# Spotify OAuth Callback Server Setup

This document describes the Spotify OAuth callback server configuration with Traefik and HTTPS.

## Overview

The OAuth callback server is implemented using Axum and runs on port 3000 by default. It's exposed via Traefik with automatic HTTPS certificates from Let's Encrypt.

## Configuration

### 1. Docker Compose Setup

The `docker-compose.yml` includes:

- **server** service: Axum OAuth callback server
- **traefik** service: Reverse proxy with automatic HTTPS

### 2. Traefik Configuration

Traefik is configured to:

- Listen on port 443 by default (HTTPS only)
- Obtain SSL certificates from Let's Encrypt using DNS-01 challenge (Hurricane Electric)
- **No port 80 required** - DNS validation only
- Route `rustify.vtvz.me` to the OAuth callback server

### 3. Required Ansible Variables

Add/update in your inventory group_vars (e.g., `_infra/ansible/inventory/main/group_vars/all.yml`):

```yaml
# Let's Encrypt email for certificate notifications
rustify_letsencrypt_email: "your-email@example.com"

# Hurricane Electric (dns.he.net) DDNS token for DNS-01 challenge
rustify_hurricane_tokens: "rustify.vtvz.me:your-generated-token"

rustify_hurricane_tokens: "rustify.vtvz.me"

rustify_env:
  SPOTIFY_REDIRECT_URI: https://rustify.vtvz.me/spotify-callback
```

#### Getting Hurricane Electric DDNS Token

Hurricane Electric uses DDNS update tokens, not username/password. Follow these steps:

1. Log into [dns.he.net](https://dns.he.net)
2. Find your domain `vtvz.me` in the list
3. Click on the domain to see all DNS records
4. Create a TXT record for `_acme-challenge.rustify.vtvz.me`:
   - Name: `_acme-challenge.rustify`
   - Type: `TXT`
   - Content: `test` (placeholder, will be updated by ACME)
5. Click the **"Enable entry for dynamic dns"** icon (looks like a key/refresh icon) next to the TXT record
6. A token will be generated - copy this token
7. Format your token as: `rustify.vtvz.me:YOUR_TOKEN` (use the domain name WITHOUT the `_acme-challenge` prefix)

**Example:**

```yaml
rustify_hurricane_tokens: "rustify.vtvz.me:a1b2c3d4e5f6g7h8i9j0"
```

**Important:** The key must be the domain requesting the certificate (`rustify.vtvz.me`), not the TXT record name. Hurricane Electric's provider will automatically add the `_acme-challenge.` prefix when updating DNS.

**For multiple domains**, separate with commas:

```yaml
rustify_hurricane_tokens: "rustify.vtvz.me:token1,api.vtvz.me:token2"
```

### 4. DNS Configuration

Ensure that `rustify.vtvz.me` points to your server's IP address:

```
A record: rustify.vtvz.me -> YOUR_SERVER_IP
```

### 5. Spotify App Configuration

Update your Spotify app settings at https://developer.spotify.com/dashboard:

1. Go to your app
2. Click "Edit Settings"
3. Add to "Redirect URIs": `https://rustify.vtvz.me:4444/spotify-callback`
4. Save
