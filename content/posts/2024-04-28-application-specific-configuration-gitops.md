---
title: Application Specific Configuration with GitOps Deployments
tags: [devops kubernetes]
date: 2024-04-28
slug: application-specific-configuration-gitops
toc: true
---

In today's fast-paced software development landscape, staying agile and efficient is crucial. One area where efficiency matters greatly is in deploying application configurations. Developers often need to make changes to configuration values based on different environments, like production, staging, or development.

## Understanding the Setup

At the core of this approach is the idea of storing application configuration values directly within the application's repository. You create YAML files for each environment, like `prod.yml` or `sandbox.yml`, and populate them with the necessary key-value pairs for different configurations.

These key-value pairs will then be used to update the relevant ConfigMap file in the GitOps repository.

## Benefits of Managing Configurations Alongside Code

The beauty of this approach is that it puts the power in the hands of developers. They can make changes to application-specific configurations directly in the repository without needing approval from DevOps teams. This speeds up the development cycle and allows teams to respond quickly to evolving requirements.

## Automating ConfigMap Patching

To automate this process, we can use a script that scans for YAML files within a `deployment-configs` directory in the application repository. If it finds any, it updates the corresponding `application_configmap.yml` file in the GitOps repository with the data from these YAML files. This means that any changes made to the YAML files in the application repository are automatically reflected in the GitOps repository, simplifying the deployment process significantly.

```bash
#!/usr/bin/env bash

if [ -d deployment-configs ]; then
    echo "Found deployment-configs directory"
    config_files=$(find deployment-configs -maxdepth 1 -type f \( -iname "$ENV.yml" -o -iname "$ENV.yaml" \))

    for file in \$config_files; do
        echo "Found \$file"

        gitops_configmap_path="gitops-deployments/.../envs/$ENV/application_configmap.yml"
        if [ -f "\$gitops_configmap_path" ]; then
            echo "Patching \$gitops_configmap_path"
            yq -i ".data = load(\"\$file\")" "\$gitops_configmap_path"
        fi
    done
fi
```

This script assumes that `yq` is installed and that an environment variable named `ENV` exists and is set to the desired environment (e.g., `prod`, `staging`). This should be defined in the relevant CI step.

This will effectily update the ConfigMap in the GitOps repository with the values from the YAML files in the application repository.
