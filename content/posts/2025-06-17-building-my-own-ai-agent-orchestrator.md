+++
title = "Building My Own AI Agent Orchestrator"
date = "2025-06-17"

[taxonomies]
tags=["coding", "devops", "ai"]

[extra]
mermaid = true
+++

AI assistants and AI-powered IDEs are great, but I wanted a solution that could handle complex tasks from start to finish, in the repositories across our organization, and that could run in the background. I wanted a tool that I could give a high-level goal to, like "refactor the auth service," and have it autonomously navigate my codebase, understand its nuances, and execute the task, seamlessly integrated with the company's self-hosted GitLab instance.

## The Vision: A Hands-Off AI Collaborator

The goal was to create a system where I could delegate complex tasks to an AI agent. The agent needed to be able to check out the right code, have the right tools and credentials, and understand project-specific instructions to get the job done. I also needed a simple web interface to submit these tasks and monitor their progress.

Tools like OpenAI Codex and Cursor Background Agents already exist, but they are only available with GitHub and come with limitations. In my case, we are using a self-hosted GitLab instance in our company, so we wanted seamless integration with our existing infrastructure, with a user-friendly wrapper and additional features.

For the purposes of this post, I'll be referring to this system as "Agent", even though we have a different internal name for it.

## The High-Level Architecture

I decided to split the system into the following primary components:

1.  **Backend Server:** A Go application that manages tasks and orchestrates the AI agents.
2.  **Web Application:** A React and TypeScript interface for creating and tracking tasks.
3.  **Runtime:** A Docker image that contains the agent and all the tools it needs to execute the tasks.

This separation of concerns keeps the architecture clean and scalable. The frontend is focused purely on user interaction, while the backend handles the heavy lifting of agent execution.

{% mermaid() %}
sequenceDiagram
participant Web App
participant Kubernetes API
participant Kubernetes Job
participant Agent Runtime
participant GitLab Repository

    Web App->>Kubernetes API: Request job creation for code execution
    Kubernetes API->>Kubernetes Job: Start job with Agent Runtime image
    Kubernetes Job->>Agent Runtime: Initialize runtime environment

    par Log streaming and runtime execution
        Agent Runtime->>Web App: Stream job logs in real time
    and
        Agent Runtime->>GitLab Repository: Clone repository
        loop Execute code tasks
            Agent Runtime->>Agent Runtime: Execute code tasks
        end
        Agent Runtime->>GitLab Repository: Push changes
    end

    Agent Runtime->>Kubernetes API: Notify completion
    Kubernetes API->>Kubernetes Job: Terminate job
    Kubernetes API->>Web App: Job terminated, execution complete

{% end %}

## The Implementation

### The Orchestrator

The heart of the system is the Go backend. When a new task is submitted, the backend doesn't run the agent itself. Instead, it dynamically creates and dispatches a **Kubernetes Job**.

I chose this approach for a few key advantages:

- **Isolation:** Each agent runs in its own sandboxed container.
- **Scalability:** Kubernetes can handle running multiple jobs in parallel.
- **Resilience:** Kubernetes provides robust error handling and reporting.
- **Observability:** Our Kubernetes clusters are already orchestrated with our observability stack, so we get metrics and logs collection out of the box.

The Go server exposes a simple REST API, constructs the Kubernetes Job, injects the necessary context (prompt, Git info, API keys), and monitors the job's status.

Each task has the following structure:

```go
type Task struct {
	ID         int       `json:"id"`
	Prompt     string    `json:"prompt"`
	Repository string    `json:"repository"`
	Branch     string    `json:"branch"`
	Status     string    `json:"status"`
	JobName    string    `json:"job_name"`
	UniqueID   string    `json:"unique_id"`
	Duration   *int64    `json:"duration"`
	CreatedAt  time.Time `json:"created_at"`
}
```

I used the [kubernetes/client-go](https://github.com/kubernetes/client-go) library to interact with the Kubernetes API. This means loading the Kubernetes config from either the in-cluster config or from a file, creating new jobs, monitoring their status, etc.

I keep track of the created tasks in an in-memory list wrapper with a `sync.Mutex` to ensure thread-safety, but this will later be replaced with a database.

### The Runtime

The Kubernetes Job runs the specific runtime Docker image. This container is the environment where the AI agent lives. It's equipped with all the tools it needs:

- Git command-line tools.
- API credentials for services like GitLab, Jira, and Coda, and Anthropic.
- Claude Code and MCP servers.

The user's prompt is passed directly as a command-line argument to the agent process inside the container.

Following is a sample Dockerfile for the runtime:

```dockerfile
FROM ubuntu:24.04

RUN apt-get update -y
RUN apt-get install -y curl gnupg git sudo pipx
RUN curl -fsSL https://deb.nodesource.com/setup_23.x | bash - && apt-get install -y nodejs

RUN useradd -m -u 1001 -s /bin/bash agent
RUN echo "agent ALL=(ALL) NOPASSWD:ALL" | tee /etc/sudoers.d/agent
USER agent

ENV PATH="/home/agent/.local/bin:/home/agent/.npm-global/bin:$PATH"

RUN mkdir -p ~/.npm-global
RUN npm config set prefix '~/.npm-global'
RUN npm install -g @anthropic-ai/claude-code

RUN pipx install uv

COPY ./docker/entrypoint.sh /entrypoint.sh

WORKDIR /src

ENTRYPOINT ["/entrypoint.sh"]
```

And the entrypoint script:

```bash
#!/usr/bin/env bash
set -euo pipefail

if [ -z "$ANTHROPIC_API_KEY" ]; then echo "Error: ANTHROPIC_API_KEY environment variable is required"; exit 1; fi
if [ -z "$GIT_REPO" ]; then echo "Error: GIT_REPO environment variable is required"; exit 1; fi
# --- snip ---

echo "Setting up Claude configuration..."
mkdir -p ~/.claude
echo '{"apiKeyHelper": "~/.claude/anthropic_key.sh"}' > ~/.claude/settings.json
cat > ~/.claude/anthropic_key.sh << EOF
echo "$ANTHROPIC_API_KEY"
EOF
chmod +x ~/.claude/anthropic_key.sh

echo "Creating MCP Servers file.."
cat > /tmp/mcp.json << EOF
{
    "mcpServers": {
      # --- snip ---
    }
}
EOF


echo "Setting up git configuration..."
git config --global user.email ...
git config --global user.name ...

echo "Cloning repository..."
if [ -n "${GIT_BRANCH:-}" ] && [ "${GIT_BRANCH:-}" != "" ]; then
    git clone https://$GITLAB_USER:$GITLAB_TOKEN@gitlab.company.com/$GIT_REPO.git --depth 1 --branch $GIT_BRANCH
else
    git clone https://$GITLAB_USER:$GITLAB_TOKEN@gitlab.company.com/$GIT_REPO.git --depth 1
fi

cd $repo_name

if [ ! -f "CLAUDE.md" ]; then echo "Warning: CLAUDE.md file not found in repository"; fi

echo "Running project setup script..."
if [ ! -f "agent-setup.sh" ]; then echo "Error: agent-setup.sh not found in repository"; exit 1; fi
bash agent-setup.sh

echo "Creating branch $branch_name..."
git checkout -b $branch_name

echo "Running claude with prompt: $prompt"
claude --dangerously-skip-permissions --mcp-config /tmp/mcp.json --verbose --output-format stream-json -p "$prompt"

echo "Pushing changes..."
git add -A
git commit -m "$title"
git push origin $branch_name

echo "Creating merge request..."

# Get the default branch (usually main or master)
default_branch=$(git remote show origin | grep "HEAD branch" | cut -d' ' -f5)
if [ -z "$default_branch" ]; then
    default_branch="main"
fi

echo "Creating merge request..."
mr_response=$(curl -s -X POST \
  -H "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"source_branch\": \"$branch_name\",
    \"target_branch\": \"$default_branch\",
    \"title\": \"$title\",
    \"description\": \"Automated changes generated by Agent\\n\\nPrompt: $prompt\",
    \"squash\": true,
    \"remove_source_branch\": true
  }" \
  "https://gitlab.company.com/api/v4/projects/$(echo $GIT_REPO | sed 's/\//%2F/g')/merge_requests")


mr_url=$(echo "$mr_response" | grep -o '"web_url":"[^"]*"' | cut -d'"' -f4)
if [ -n "$mr_url" ]; then
    echo "Merge request created successfully: $mr_url"
else
    echo "Failed to create merge request. Response: $mr_response"
    exit 1
fi
```

As you can see, the entrypoint script takes care of the following:

- Setting up the GitLab credentials.
- Setting up Claude Code & MCP servers.
- Cloning the repository.
- Running the project-specific setup script. This is needed to install dependencies, run database migrations, or set up any other project-specific tooling.
- Creating a new branch.
- Running the agent with the prompt.
- Pushing the changes to the repository.
- Creating a merge request.

### The Frontend

The frontend is a straightforward application built with Vite & React. A form allows the users to input the three key pieces of information for a task:

1.  **Prompt:** The high-level instruction for the agent.
2.  **Repository:** The path to the Git repository.
3.  **Branch:** The specific branch to check out (optional).

Once a task is submitted, it appears in a table that shows its status and auto-refreshes. Clicking a task streams the logs directly from the Kubernetes pod, which is crucial for seeing what the agent is doing.

## Project-Specific Context

One of the most powerful features is how the system understands the context of a specific project. It does this by looking for two special files in the root of the target repository:

1.  `agent-setup.sh`: A shell script the agent executes first. I use this to install dependencies (e.g. `pnpm install`), run database migrations, or set up any other project-specific tooling.

2.  `CLAUDE.md`: A Markdown file with instructions for the AI model itself—a "meta-prompt" that guides the agent. Here, I can specify which test commands to use, coding style guidelines, or things the agent _should not_ do.

This mechanism makes the system incredibly flexible without requiring any changes to the orchestrator itself.

## A User's Journey

Let's walk through a typical use case.

1.  **The Request:** I want to add a health check endpoint to my new web service.
2.  **The Setup:** In the service's repository, I create a `CLAUDE.md` file for general-purpose project instructions that says, "This project uses Go. All new endpoints should be added in `server.go` and include a unit test."
3.  **The Submission:** I go to the web UI and create a new task with the prompt, "Add a `/health` endpoint that returns a 200 OK status," along with the repository and branch.
4.  **The Orchestration:** The Go backend creates a Kubernetes job.
5.  **The Execution:** The agent starts, clones the repo, reads the `CLAUDE.md` file, and then edits `server.go` to add the endpoint and a corresponding test.
6.  **The Monitoring:** On the UI, I can see the status change from `pending` to `running` and watch the logs.
7.  **The Completion:** Once the agent commits and pushes the code, the job completes, and the UI updates to `completed`.

## Conclusion

Building this AI agent orchestrator has been a fascinating project, combining Go, React, Kubernetes, and LLMs into a practical tool. Using Kubernetes for execution makes the system scalable and robust, and the project-specific context files make the agent highly adaptable.

This project has shown immense value to the organization in both cost and time savings. It's sometimes best used as the first step toward implementing a complex feature, and at other times it manages to deliver the feature in a fraction of the time.
