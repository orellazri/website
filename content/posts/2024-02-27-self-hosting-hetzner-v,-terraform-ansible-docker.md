---
title: "Self Hosting On a Hetzner VM: Terraform, Ansible, Docker"
tags: [devops, selfhosting]
date: 2024-02-27
slug: self-hosting-hetzner-vm-terraform-ansible-docker
toc: true
---

I started my self-hosting journey a few years back with a single desktop machine serving as both my NAS and the host for all the services I was running.

I found out the hard way, that sometimes it's not the best decision to have the important things that need to be always available running in my home, since they are doomed to experience downtime (power failures, ISP maintenance, and whatnot).

After deciding that I want my critical services (like [paperless](https://docs.paperless-ngx.com/) for document management, [Nextcloud](https://nextcloud.com/) for cloud storage, and [Immich](https://immich.app/) for my photos and videos) to endure as little downtime as possible, I started looking for a relatively cheap solution - a VM and block storage.

[Hetzner](https://www.hetzner.com/) proved to be the the best solution I could find. They have unbeatable prices for ARM based VMs, and storage boxes that you can mount via CIFS, for as little as ~3€/mo.

I also took this opportunity to get more familiar with Terraform, and to build my infrastructure declaratively.

## Using Terraform to provision cloud resources

After signing up to Hetzner, you'll need to log in to your cloud dashboard and create a project. A project is a separation that you can have between different, well, projects. This is a good time to pick a name for this rabbithole we're going down in.

Then, go ahead and generate an API token (under Security), name it `terraform`, and give it both read and write permissions.

Create a repository and inside a `terraform` directory. The `main.tf` file will look similar to this:

```hcl
terraform {
  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "1.41.0"
    }
  }
}

provider "hcloud" {
  token = var.hcloud_token
}

resource "hcloud_ssh_key" "personal" {
  name       = "personal"
  public_key = file(var.ssh_public_key_path)
}

resource "hcloud_firewall" "server" {
  name = "crux-server-firewall"
  rule {
    description = "ssh"
    direction   = "in"
    protocol    = "tcp"
    port        = "22"
    source_ips  = ["0.0.0.0/0"]
  }
  rule {
    description = "http"
    direction   = "in"
    protocol    = "tcp"
    port        = "80"
    source_ips  = ["0.0.0.0/0"]
  }
  rule {
    description = "https"
    direction   = "in"
    protocol    = "tcp"
    port        = "443"
    source_ips  = ["0.0.0.0/0"]
  }
}

resource "hcloud_server" "server" {
  name         = "project-server"
  image        = "ubuntu-22.04"
  server_type  = "cax11"
  location     = "fsn1"
  ssh_keys     = [hcloud_ssh_key.personal.id]
  firewall_ids = [hcloud_firewall.server.id]

  public_net {
    ipv6_enabled = false
  }
}

output "server_ip" {
  value = hcloud_server.server.ipv4_address
}
```

This file basically provisions your firewall and VM. In my case, I made sure to add my public SSH key to the VM so I can SSH into it as soon as it's done provisioning. I also opened ports 80 and 443 in the firewall so I can have my domain name pointing to the VM.

Make sure to change the values accordingly (such as the name and server type for your VM).

The other files you'll need is `variables.tf`:

```hcl
variable "hcloud_token" {}

variable "ssh_public_key_path" {
  default = "~/.ssh/personal.pub"
}
```

`hcloud_token` is your API token, which will of course not be committed into your git repository, hence it's empty. When running Terraform, you can add a `--var-file=secrets.tfvars` which will point to a separate variables file that will contain the actual API token and will not be checked in to the repo.

In my setup, I also added the `remote-exec` and `local-exec` provisioners so it runs my Ansible playbook automatically after the VM is up, but I left it out as an exercise to the reader ™.

Now it should be as simple as running `terraform init` to initialize the directory, then `terraform plan` to view the planned changes, and `terraform apply --var-file=secrets.tfvars` to apply the changes and provision the resources.

## Using Ansible to set up the VM

I'm using Ansible to manage the configuration of my VM - my bash scripts, Docker compose files, cronjobs, packages, etc.

Create a directory named `ansible` next to the `terraform` directory. Inside it, create a playbook with a name of your choice (`site.yml`, `vm.yml`, or whatever):

```yaml
- name: Configure VM
  hosts: all
  gather_facts: false
  become: true
  roles:
    - vm
```

This basically calls the `vm` roles. Go ahead and create the following file: `roles/vm/tasks/main.yml`

Now, it's really up to you to build this role to your liking and configure anything you dream of.
Me, personally? I use it to configure basic stuff (user, group, directories, packages), install Docker & lazydocker, disable root & password ssh authentication and so on.

Here are some useful tasks that you may find yourself using as well:

### Configure sshd

```yaml
- name: Configure sshd login
  lineinfile:
    dest: /etc/ssh/sshd_config
    regexp: "{{ item.regexp }}"
    line: "{{ item.line }}"
    state: present
  loop:
    - { regexp: "^PermitRootLogin", line: "PermitRootLogin no" }
    - { regexp: "^PasswordAuthentication", line: "PasswordAuthentication no" }
    - { regexp: "^KbdInteractiveAuthentication", line: "KbdInteractiveAuthentication no" }

- name: Restart SSH server
  service:
    name: sshd
    state: restarted
```

### Install Docker

```yaml
- name: Check if Docker is installed
  command: docker --version
  register: docker_valid
  changed_when: false
  ignore_errors: true

- name: Install Docker
  when: docker_valid.failed
  block:
    - name: Install required system packages
      apt:
        name:
          - ca-certificates
          - curl
          - gnupg
          - lsb-release
        state: present
        update_cache: true

    # noqa yaml[line-length]
    - name: Set up GPG key and repo
      shell: |
        mkdir -p /etc/apt/keyrings
        curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
        echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
        https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"\
        | tee /etc/apt/sources.list.d/docker.list > /dev/null
      tags: skip_ansible_lint

    - name: Install Docker packages
      apt:
        name:
          - docker-ce
          - docker-ce-cli
          - containerd.io
          - docker-compose-plugin
        state: present
        update_cache: true
```

### Install lazydocker

```yaml
- name: Check if lazydocker is installed
  command: lazydocker --version
  register: lazydocker_valid
  changed_when: false
  ignore_errors: true

- name: Install lazydocker
  when: lazydocker_valid.failed
  block:
    - name: Download lazydocker
      get_url:
        url: https://github.com/jesseduffield/lazydocker/releases/download/v0.23.1/lazydocker_0.23.1_Linux_arm64.tar.gz
        dest: /tmp/lazydocker.tar.gz

    - name: Extract lazydocker
      unarchive:
        src: /tmp/lazydocker.tar.gz
        dest: /tmp
        remote_src: true

    - name: Copy lazydocker to /usr/local/bin
      copy:
        src: /tmp/lazydocker
        dest: /usr/local/bin/lazydocker
        mode: "0755"
        remote_src: true

    - name: Remove temporary lazydocker files
      file:
        path: "{{ item }}"
        state: absent
      loop:
        - /tmp/lazydocker.tar.gz
        - /tmp/lazydocker
```

### Synchronize docker compose directory

```yaml
- name: Synchronize stacks
  template:
    src: "{{ item }}"
    dest: "/home/<user>/stacks/{{ item | basename }}"
    owner: <user>
    mode: "0644"
  with_fileglob:
    - templates/stacks/*.yml
  tags: stacks
```

### Mount Hetzner Storage Box

```yaml
- name: Create storage box mount directory
  file:
    path: /mnt/Storage
    state: directory
    owner: <user>
    mode: "0755"

- name: Mount storage box
  mount:
    path: /mnt/Storage
    src: "{{ hcloud_storage_box_path }}"
    fstype: cifs
    opts: "iocharset=utf8,rw,user={{ hcloud_storage_box_username }},pass={{ hcloud_storage_box_password }},uid=1000,gid=1000,file_mode=0660,dir_mode=0770"
    state: mounted
```

It's really handy to use ansible-vault to manage secret variables.

## Crafting Docker compose files

Your next step will be to actually run the services you want. For that, you'll need Docker compose files. I like to keep them in the Ansible role `templates/stacks` directory, and use Ansible's templating engine to fill in secret variables such as tokens and passwords.

Here are some compose files that can come in handy (make sure to change the variables and volume mounts accordingly):

### NGINX Proxy Manager

```yaml
version: "3.8"
services:
  app:
    image: "jc21/nginx-proxy-manager:latest"
    container_name: nginx-proxy-manager
    restart: unless-stopped
    network_mode: host
    volumes:
      - /home/<user>/volumes/nginx-proxy-manager/data:/data
      - /home/<user>/volumes/nginx-proxy-manager/letsencrypt:/etc/letsencrypt
```

### Nextcloud

```yaml
version: "3.8"
services:
  mariadb:
    image: mariadb:10.6
    container_name: nextcloud-mariadb
    command: --transaction-isolation=READ-COMMITTED --log-bin=binlog --binlog-format=ROW
    restart: unless-stopped
    volumes:
      - /home/<user>/volumes/nextcloud/mariadb:/var/lib/mysql:Z
    environment:
      - MYSQL_ROOT_PASSWORD={{ nextcloud_mysql_password }}
      - MYSQL_PASSWORD={{ nextcloud_mysql_password }}
      - MYSQL_DATABASE=nextcloud
      - MYSQL_USER=nextcloud
      - MARIADB_AUTO_UPGRADE=1
      - MARIADB_DISABLE_UPGRADE_BACKUP=1

  redis:
    image: redis:alpine
    container_name: nextcloud-redis
    restart: unless-stopped

  nextcloud:
    image: lscr.io/linuxserver/nextcloud:latest
    container_name: nextcloud
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=<timezone>
    volumes:
      - /home/<user>/volumes/nextcloud:/config
      - /mnt/<nextcloud dir>:/data
    ports:
      - 8082:443
    depends_on:
      - mariadb
      - redis
    restart: unless-stopped
```

### Accessing the VM from outside

The safest option to access your VM from outside the cloud network is using a VPN. You can self host Wireguard with `wg-easy` or use Tailscale (recommended).

If you want your services to be publicly accessible, you'll need to set up a reverse proxy which will map subdomains to your services which are exposed on certain ports (for example, to access your Nextcloud instance, you'll visit `nextcloud.mylab.com` instead of `mylab.com:8082`).

I do a mix of both. I like using Tailscale to access services that I don't need exposed to the internet, and that I don't mind connecting to the VPN for, like Netdata, and I use Nginx Proxy Manager as a reverse proxy for other services that I want to access from anywhere without a VPN.

FYI, there are other solutions which I won't cover here like Cloudflare tunnels. Do your research if this seems right for you.

#### Reverse proxy

First you'll need to purchase a domain. I recommend heading over to [TLD-List](https://tld-list.com/) to look for a cheap TLD (you probably don't need `.com` or `.ai` for your cheap lab).

After that, I like to change the DNS nameserver for the domain from the domain registrar's to Cloudflare, simply because I like their UI and their API.

Set an A record for `@` (root) to your VM's IP address, and set a CNAME record for `*` to point to `@`, so any subdomain will also point to the VM.

Then in your reverse proxy of choice, you can map each subdomain you want to a service, along with an SSL certificate.

## Final words

I hope you found this writeup useful. I tried to only write about the general gist of things without going into many details, but also without being too brisk and missing critical points. Self-hosting can be intimidating at first, but once you experiment with it, and try things out, it's not too scary.
