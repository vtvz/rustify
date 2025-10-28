# Deployment Setup Guide

This guide explains how to configure GitHub Actions for automated deployment.

## Prerequisites

- GitHub repository with Actions enabled
- SSH access to your deployment server
- Docker images pushed to GitHub Container Registry (ghcr.io)

## GitHub Secrets Configuration

You need to add the following secrets to your repository:

**Settings → Secrets and variables → Actions → New repository secret**

### Required Secrets

#### 1. `ANSIBLE_HOSTS_INI`
Base64-encoded Ansible hosts inventory file.

**Generate:**
```bash
cat _infra/ansible/inventory/main/hosts.ini | base64 -w 0
```

Copy the output and paste it as the secret value.

---

#### 2. `ANSIBLE_GROUP_VARS_ALL`
Base64-encoded Ansible group variables file containing all configuration.

**Generate:**
```bash
cat _infra/ansible/inventory/main/group_vars/all.yml | base64 -w 0
```

Copy the output and paste it as the secret value.

---

#### 3. `ANSIBLE_SSH_PRIVATE_KEY`
SSH private key for accessing the deployment server.

**Generate (if you don't have one):**
```bash
ssh-keygen -t ed25519 -C "github-actions-deploy" -f ~/.ssh/github_deploy_key
```

Then copy the **private key** to the server:
```bash
ssh-copy-id -i ~/.ssh/github_deploy_key.pub root@your-server.com
```

**Add to GitHub:**
```bash
cat ~/.ssh/github_deploy_key
```

Copy the entire output (including `-----BEGIN OPENSSH PRIVATE KEY-----` and `-----END OPENSSH PRIVATE KEY-----`) and paste as the secret value.

---

## Repository Permissions

Ensure GitHub Actions has permission to push to GitHub Container Registry:

1. Go to **Settings → Actions → General**
2. Under "Workflow permissions", select **Read and write permissions**
3. Click **Save**

---

## Running the Deployment

1. Go to **Actions** tab in your repository
2. Select **Deploy to Production** workflow
3. Click **Run workflow**
4. Select the branch (usually `master`)
5. Click **Run workflow**

The workflow will:
1. ✅ Validate code (fmt, clippy, tests)
2. 🐳 Build Docker images
3. 📦 Push to ghcr.io
4. 🚀 Deploy with Ansible

---

## Troubleshooting

### Build fails on validation
Run locally before pushing:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features --no-deps -- -D warnings
cargo test
```

### SSH connection fails
- Ensure the SSH key has been added to the server's `~/.ssh/authorized_keys`
- Check server firewall allows SSH connections from GitHub Actions IPs
- Verify the hostname/IP in `ANSIBLE_HOSTS_INI` is correct

### Ansible fails
- Check that inventory files are properly formatted
- Verify all required variables are present in `group_vars/all.yml`
- Review Ansible playbook logs in Actions output

### Docker pull fails on server
- Ensure the server can access ghcr.io
- Verify `rustify_docker_registry_password` in group_vars is a valid GitHub token with `read:packages` permission

---

## Security Notes

⚠️ **Important:**
- Never commit unencrypted secrets to the repository
- Regularly rotate SSH keys and API tokens
- Review Actions logs carefully - they should not expose secrets
- The Ansible `--diff` flag is intentionally disabled to prevent secret exposure in logs