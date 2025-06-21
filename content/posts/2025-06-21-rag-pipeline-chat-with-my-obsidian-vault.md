+++
title = "RAG Pipeline to Chat with My Obsidian Vault"
date = "2025-06-21"

[taxonomies]
tags=["coding", "ai"]
+++

My Obsidian vault has grown significantly over time with snippets, project documentation, meeting notes, and random thoughts. I wanted a conversational way to interact with my notes - to ask natural language questions like "How did I fix that Git submodule issue?" or "What were the main challenges in the refactor?" and get contextual answers from my notes.

Rather than relying on cloud-based solutions, I decided to see if I could build my own RAG (Retrieval Augmented Generation) pipeline that runs entirely locally, keeping my notes private and giving me full control over the implementation - mainly for fun.

You can find the source code [here](https://github.com/orellazri/mdrag).

## A Local RAG Pipeline

I built **mdrag** (Markdown RAG), a Rust-based CLI tool that creates a searchable vector database from markdown files and uses local AI models to generate responses. The name might not be the most creative, but it gets the job done!

The system works in two phases:

1. **Embedding**: Process all markdown files, chunk them intelligently, and generate vector embeddings
2. **Querying**: Take natural language queries, find similar content, and use a local LLM to generate answers

## The Implementation

These are the main interesting dependencies I've used:

- [rusqlite](https://docs.rs/rusqlite/latest/rusqlite/) and [sqlite-vec](https://docs.rs/sqlite-vec/latest/sqlite_vec/) for using SQLite as a vector database
- [ollama-rs](https://docs.rs/ollama-rs/latest/ollama_rs/) - This allows me to interact with a local Ollama instance in an idiomatic way instead of writing and parsing plain requests
- [zerocopy](https://docs.rs/zerocopy/latest/zerocopy/) - For zero-cost memory manipulation. This is used to reference data in and out of SQLite without memory allocations.

### First step: Markdown Parsing and Chunking

One of the trickiest parts was intelligently chunking markdown content. Simply splitting by character count would break apart related information. Instead, I implemented a parser that is slightly more sophisticated, though it could still be improved significantly:

1. **Detects the primary header level** used in each document
2. **Splits content by those headers**, keeping related information together
3. **Handles edge cases** like content before the first header
4. **Further splits large chunks** while respecting paragraph boundaries

```rust
pub fn parse(contents: &str, filename: &str) -> Result<Vec<String>> {
    // Find the first header type used in the document
    let header_regex = Regex::new(r"^(#{1,6})\s")?;

    let first_header_level = contents
        .lines()
        .find_map(|line| header_regex.captures(line).map(|caps| caps[1].to_string()));

    // Split by the detected header level or treat as single chunk
    // Each chunk gets prefixed with the filename for context
}
```

This approach ensures that a section like "## Git Snippets" stays together with all its related commands, making retrieval more meaningful.

### Second step: Vector Embeddings with SQLite

Instead of using a dedicated vector database, I leveraged **sqlite-vec**, a SQLite extension that adds vector similarity search capabilities. This keeps the entire system self-contained-no additional services to run or manage.

This extension can be loaded with a short unsafe block:

```rust
unsafe {
    sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
}
```

And then we can create a table with the following schema:

```rust
pub fn new(conn: &'a Connection, ollama: &'a Ollama) -> Result<Self> {
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS file_embeddings USING vec0(
            path TEXT,
            contents TEXT,
            embedding FLOAT[768]
        )",
        [],
    )?;

    Ok(Self { conn, ollama })
}
```

The embedding process walks through all markdown files in the vault, checks if they've already been processed (to avoid redundant work), and generates 768-dimensional vectors using Ollama's [nomic-embed-text-v1.5](https://huggingface.co/nomic-ai/nomic-embed-text-v1.5) model. Using a different model means that the embedding vector size will probably need to be changed.

### Third step: Querying the vector database

In querying mode, a user enters a natural language query, and the system generates an embedding for it, then uses the vector database to find the most similar chunks. Finally, it uses the LLM to generate a response.

The interesting part where we fetch the similar vectors from SQLite is this:

```rust
let mut stmt = self.conn.prepare(
    "SELECT contents
    FROM file_embeddings
    WHERE embedding MATCH ?1
    AND k = ?2
    ORDER BY distance",
)?;

let results = stmt
    .query_map((query_embedding[0].as_bytes(), k as i32), |row| {
        Ok(row.get(0)?)
    })?
    .collect::<Result<Vec<String>, _>>()?;
```

We use the `embedding MATCH` clause to find the most similar vectors to the query embedding, and return the contents (the original text that was embedded). This then lets the LLM generate a response based on the context of the most similar chunks.

For the actual text generation, I integrated with **Ollama**, which provides a simple API for running various models locally. I chose `gemma3:1b` because I wanted an extremely lightweight model to test things quickly.

```rust
// Embed the query
let results = embedder.search(&query, 5).await?;

// Build a prompt
let model = "gemma3:1b".to_string();
let options = ModelOptions::default()
    .temperature(0.2)
    .top_k(25)
    .top_p(0.25);
let mut prompt = format!(
    "Answer the following user question: {query}\n\nUsing the following information for context:\n\n"
);
results
    .iter()
    .for_each(|result| prompt.push_str(&format!("\n\n{}", result)));

// Generate a streaming response
let mut stream = ollama
    .generate_stream(GenerationRequest::new(model, prompt).options(options))
    .await?;

// Stream the response to stdout
let mut stdout = io::stdout();
while let Some(res) = stream.next().await {
    let responses = res?;
    for resp in responses {
        stdout.write_all(resp.response.as_bytes()).await?;
        stdout.flush().await?;
    }
}
```

The streaming interface provides immediate feedback, making the tool feel responsive even when the model is working on complex queries.

## Usage

Let's walk through a typical interaction:

1. **Initial Setup**: I run `mdrag embed ~/vault` to process all my markdown files. This creates embeddings for each chunk and stores them in the SQLite database.

2. **Querying**: When I want to find something, I run `mdrag search "how to debug git ssh issues"`. The system:

   - Generates an embedding for my query
   - Finds the most similar chunks from my notes
   - Constructs a prompt with the query and context
   - Streams the AI-generated response

3. **Getting Results**: Instead of just showing me raw search results, I get a conversational answer that synthesizes information from multiple sources in my vault.

For example, querying about Git SSH debugging would pull relevant snippets from my Git.md file and provide a comprehensive answer with the exact commands I've used before.

The system can also be modified to act as a chatbot, where users can ask follow-up questions and the system will use the context of the previous conversation for responses.

## Local-First Approach

Running everything locally has several advantages:

- **Privacy**: My notes never leave my machine - this is crucial for vaults that contain sensitive information like work-related notes
- **Cost**: No API costs for embeddings or inference (other than electricity 😅)
- **Reliability**: Works offline and doesn't depend on external services

The performance is surprisingly good. Initial embedding takes a few seconds, and subsequent queries are nearly instantaneous. The SQLite vector search is remarkably fast, and the 1B parameter model provides good results without requiring expensive hardware.
