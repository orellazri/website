+++
title = "eBPF Development on Apple Silicon Macs using Lima"
date = "2024-07-07"

[taxonomies]
tags=["coding"]
+++

I've been working on a project that involves eBPF, and I've been doing all of my development on an Apple Silicon Mac. I wanted to share how I've set up my development environment.

## The issue

eBPF is a Linux kernel feature, and as such, it requires a Linux kernel to run. This means that you can't run eBPF programs on macOS directly, so you'll have to use a virtual machine. However, using a virtual machine on an Apple Silicon Mac is not as straightforward as on an Intel Mac.

## Lima

[Lima](https://lima-vm.io/) is a program that launches virtual machines with automatic file sharing and port forwarding (similar to WSL2 in Windows). It's a great tool for running Linux VMs on both Apple Silicon and Intel Macs, and is using QEMU under the hood.

I used it since it supports configuration via a simple YAML file, which makes it easy to set up a VM with the necessary development environment for eBPF programming.

To install Lima, run:

```bash
brew install lima
```

Then, in your project directory, create a YAML file (I called mine `lima.yaml`) with the following contents:

```yaml
images:
  - location: 'https://cloud-images.ubuntu.com/releases/24.04/release-20240423/ubuntu-24.04-server-cloudimg-arm64.img'
    arch: aarch64

mounts:
  - location: '~'
    writable: true

provision:
  - mode: system
    script: |
      apt-get install -y apt-transport-https ca-certificates curl clang llvm jq
      apt-get install -y libelf-dev libpcap-dev libbfd-dev binutils-dev build-essential make
      apt-get install -y linux-tools-common linux-tools-$(uname -r)
      apt-get install -y bpfcc-tools
      apt-get install -y python3-pip
      apt-get install -y libbpf-dev
```

This YAML files contains the VM configuration. In my case, I pointed it to use an Ubuntu 24.04 image, and I installed the necessary tools for eBPF development in the provision script.

To start the VM, run:

```bash
limactl start --name ebpf lima.yaml
```

This will start a new VM with the name `ebpf` using the configuration in the `lima.yaml` file.

To enter the VM, run:

```bash
limactl shell ebpf
```

And voila, you're now inside a Linux VM on your Apple Silicon Mac, ready to do eBPF development!
