---
title: Deploying an AWS EKS Cluster With Terraform
tags: [devops, kubernetes]
date: 2024-09-03
slug: deploying-aws-eks-cluster-with-terraform
---

In this post, we'll walk through the process of deploying an Amazon EKS (Elastic Kubernetes Service) cluster using Terraform. We'll cover everything from setting up the basic infrastructure to configuring the cluster and its node groups. Let's dive in!

## Table of Contents

1. [Introduction](#introduction)
2. [Setting Up the Terraform Configuration](#setting-up-the-terraform-configuration)
3. [Networking Configuration](#networking-configuration)
4. [EKS Cluster Configuration](#eks-cluster-configuration)
5. [Node Group Configuration](#node-group-configuration)
6. [IAM Permissions](#iam-permissions)
7. [Conclusion](#conclusion)

## Introduction

Amazon EKS is a managed Kubernetes service that makes it easy to run Kubernetes on AWS without needing to install and operate your own Kubernetes control plane. In this guide, we'll use Terraform to automate the deployment of an EKS cluster, including all necessary networking and security configurations.

## Setting Up the Terraform Configuration

First, let's set up our main Terraform configuration file. This file will define the AWS provider and some default tags for our resources.

Create a `main.tf` file, and inside it, add the following configuration:

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = "us-east-1"

  assume_role {
    role_arn = "arn:aws:iam::<ACCOUNT_ID>:role/<ROLE>"
  }

  default_tags {
    tags = {
      Managed-By  = "Terraform"
      EKS-Cluster = "<CLUSTER_NAME>"
    }
  }
}
```

In this configuration:

- We specify the AWS provider and its version.
- We set the AWS region to `us-east-1`.
- We use an IAM role for authentication (replace `<ACCOUNT_ID>` with your actual AWS account ID and `<ROLE>` with your actual role name).
- We define default tags that will be applied to all resources created by Terraform.

After creating this file, run `terraform init` to initialize the Terraform configuration.

## Networking Configuration

Next, let's set up the networking infrastructure for our EKS cluster. This includes creating a VPC, subnets, internet gateway, NAT gateway, and route tables.

Create a new file named `networking.tf` and add the following configuration:

```hcl
resource "aws_vpc" "main" {
  cidr_block = "10.43.0.0/22"

  tags = {
    Name = "<CLUSTER_NAME>-main-vpc"
  }
}

resource "aws_internet_gateway" "igw" {
  vpc_id = aws_vpc.main.id

  tags = {
    Name = "<CLUSTER_NAME>-igw"
  }
}

# Create private and public subnets
resource "aws_subnet" "private-us-east-1a" {
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.43.0.0/24"
  availability_zone = "us-east-1a"

  tags = {
    "Name"                                 = "<CLUSTER_NAME>-private-us-east-1a"
    "kubernetes.io/role/internal-elb"      = "1"
    "kubernetes.io/cluster/<CLUSTER_NAME>" = "owned"
    "SubnetType"                           = "Private"
  }
}

resource "aws_subnet" "private-us-east-1b" {
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.43.1.0/24"
  availability_zone = "us-east-1b"

  tags = {
    "Name"                                 = "<CLUSTER_NAME>-private-us-east-1b"
    "kubernetes.io/role/internal-elb"      = "1"
    "kubernetes.io/cluster/<CLUSTER_NAME>" = "owned"
    "SubnetType"                           = "Private"
  }
}

resource "aws_subnet" "public-us-east-1a" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.43.2.0/24"
  availability_zone       = "us-east-1a"
  map_public_ip_on_launch = true

  tags = {
    "Name"                                 = "<CLUSTER_NAME>-public-us-east-1a"
    "kubernetes.io/role/elb"               = "1"
    "kubernetes.io/cluster/<CLUSTER_NAME>" = "owned"
  }
}

resource "aws_subnet" "public-us-east-1b" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.43.3.0/24"
  availability_zone       = "us-east-1b"
  map_public_ip_on_launch = true

  tags = {
    "Name"                                 = "<CLUSTER_NAME>-public-us-east-1b"
    "kubernetes.io/role/elb"               = "1"
    "kubernetes.io/cluster/<CLUSTER_NAME>" = "owned"
  }
}

# Set up NAT gateway
resource "aws_eip" "nat" {
  domain = "vpc"

  tags = {
    Name = "<CLUSTER_NAME>-nat"
  }
}

resource "aws_nat_gateway" "nat" {
  allocation_id = aws_eip.nat.id
  subnet_id     = aws_subnet.public-us-east-1a.id

  tags = {
    Name = "<CLUSTER_NAME>-nat"
  }

  depends_on = [aws_internet_gateway.igw]
}

# Create route tables
resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.nat.id
  }

  # ... (other routes)

  tags = {
    Name = "<CLUSTER_NAME>-private-route-table"
  }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.igw.id
  }

  tags = {
    Name = "<CLUSTER_NAME>-public-route-table"
  }
}

# Associate route tables with subnets
resource "aws_route_table_association" "private-us-east-1a" {
  subnet_id      = aws_subnet.private-us-east-1a.id
  route_table_id = aws_route_table.private.id
}

resource "aws_route_table_association" "private-us-east-1b" {
  subnet_id      = aws_subnet.private-us-east-1b.id
  route_table_id = aws_route_table.private.id
}

resource "aws_route_table_association" "public-us-east-1a" {
  subnet_id      = aws_subnet.public-us-east-1a.id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "public-us-east-1b" {
  subnet_id      = aws_subnet.public-us-east-1b.id
  route_table_id = aws_route_table.public.id
}

# Create security group
resource "aws_security_group" "internal" {
  name_prefix = "<CLUSTER_NAME>-internal-security-group"
  description = "Allow traffic within the internal network"
  vpc_id      = aws_vpc.main.id

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "<CLUSTER_NAME>-internal-security-group"
  }
}
```

This configuration sets up:

- A VPC with a CIDR block of 10.43.0.0/22 (You can use any CIDR block you prefer)
- Public and private subnets in two availability zones
- An Internet Gateway for public internet access
- A NAT Gateway for outbound internet access from private subnets
- Route tables for public and private subnets
- A security group allowing HTTPS traffic from a specific CIDR range

## EKS Cluster Configuration

Now, let's define our EKS cluster back in the `main.tf` file:

```hcl
resource "aws_eks_cluster" "cluster" {
  name     = "<CLUSTER_NAME>"
  role_arn = aws_iam_role.cluster.arn

  vpc_config {
    subnet_ids = [
      aws_subnet.private-us-east-1a.id,
      aws_subnet.private-us-east-1b.id,
      aws_subnet.public-us-east-1a.id,
      aws_subnet.public-us-east-1b.id
    ]
    endpoint_private_access = true
    endpoint_public_access  = false
    security_group_ids      = [aws_security_group.internal.id]
  }

  access_config {
    authentication_mode = "API_AND_CONFIG_MAP"
  }

  depends_on = [aws_iam_role_policy_attachment.cluster-AmazonEKSClusterPolicy]
}
```

This configuration:

- Creates an EKS cluster named `<CLUSTER_NAME>`
- Uses both private and public subnets
- Enables private endpoint access and disables public endpoint access
- Uses the internal security group we created earlier
- Sets up API server authentication using both Kubernetes API and ConfigMap

## Node Group Configuration

Next, let's set up the EKS node group. Create a new file named `nodes.tf` and add the following configuration:

```hcl
resource "aws_launch_template" "eks-with-disks" {
  name = "eks-with-disks"

  key_name = "<KEY_PAIR_NAME>"

  block_device_mappings {
    device_name = "/dev/xvda"

    ebs {
      volume_size = 50
      volume_type = "gp3"
    }
  }

  tag_specifications {
    resource_type = "instance"

    tags = {
      Name = "<CLUSTER_NAME>-node"
    }
  }
}

resource "aws_eks_node_group" "private-nodes" {
  cluster_name    = aws_eks_cluster.cluster.name
  node_group_name = "<CLUSTER_NAME>-private-nodes"
  node_role_arn   = aws_iam_role.nodes.arn

  subnet_ids = [
    aws_subnet.private-us-east-1a.id,
    aws_subnet.private-us-east-1b.id
  ]

  capacity_type  = "ON_DEMAND"
  instance_types = ["t3a.large"]

  scaling_config {
    desired_size = 3
    max_size     = 3
    min_size     = 3
  }

  launch_template {
    name    = aws_launch_template.eks-with-disks.name
    version = aws_launch_template.eks-with-disks.latest_version
  }

  depends_on = [
    aws_iam_role_policy_attachment.nodes-AmazonEKSWorkerNodePolicy,
    aws_iam_role_policy_attachment.nodes-AmazonEKS_CNI_Policy,
    aws_iam_role_policy_attachment.nodes-AmazonEC2ContainerRegistryReadOnly,
  ]
}
```

This configuration:

- Creates a launch template for EC2 instances with a 50GB gp3 EBS volume
- Sets up a node group with 3 t3a.large instances in the private subnets
- Uses On-Demand instances for the node group

## IAM Permissions

Finally, let's set up the necessary IAM roles and policies. Create a new file named `permissions.tf` and add the following configuration:

```hcl
resource "aws_iam_role" "cluster" {
  name = "<CLUSTER_NAME>-eks-cluster"

  assume_role_policy = <<POLICY
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Service": "eks.amazonaws.com"
      },
      "Action": "sts:AssumeRole"
    }
  ]
}
POLICY
}

resource "aws_iam_role_policy_attachment" "cluster-AmazonEKSClusterPolicy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
  role       = aws_iam_role.cluster.name
}

resource "aws_iam_role" "nodes" {
  name = "<CLUSTER_NAME>-eks-node-group-nodes"

  assume_role_policy = jsonencode({
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ec2.amazonaws.com"
      }
    }]
    Version = "2012-10-17"
  })
}

resource "aws_iam_role_policy_attachment" "nodes-AmazonEKSWorkerNodePolicy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
  role       = aws_iam_role.nodes.name
}

resource "aws_iam_role_policy_attachment" "nodes-AmazonEKS_CNI_Policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.nodes.name
}

resource "aws_iam_role_policy_attachment" "nodes-AmazonEC2ContainerRegistryReadOnly" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
  role       = aws_iam_role.nodes.name
}

resource "aws_iam_policy" "ebs_csi_driver" {
  name        = "ebs_csi_driver_policy"
  description = "Policy for EC2 Instances to access Elastic Block Store"

  policy = jsonencode({
    "Version" : "2012-10-17",
    "Statement" : [
      {
        "Effect" : "Allow",
        "Action" : [
          "ec2:AttachVolume",
          "ec2:CreateSnapshot",
          "ec2:CreateTags",
          "ec2:CreateVolume",
          "ec2:DeleteSnapshot",
          "ec2:DeleteTags",
          "ec2:DeleteVolume",
          "ec2:DescribeInstances",
          "ec2:DescribeSnapshots",
          "ec2:DescribeTags",
          "ec2:DescribeVolumes",
          "ec2:DetachVolume"
        ],
        "Resource" : "*"
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "nodes-AmazonEBS_CSI_Driver" {
  policy_arn = aws_iam_policy.ebs_csi_driver.arn
  role       = aws_iam_role.nodes.name
}
```

This configuration:

- Creates IAM roles for the EKS cluster and node group
- Attaches necessary policies to these roles, including the EKS cluster policy, worker node policy, CNI policy, and ECR read-only policy
- Creates a custom policy for the EBS CSI driver and attaches it to the node role

## Conclusion

In this guide, we've walked through the process of deploying an Amazon EKS cluster using Terraform. We've covered setting up the basic infrastructure, configuring the EKS cluster, setting up node groups, and managing IAM permissions.

By using Terraform, we can version control our infrastructure and easily replicate this setup across different environments. Remember to replace the placeholders in the configuration files with your actual values before running Terraform commands.

Happy clustering!
