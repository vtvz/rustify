<div align="center">
  <img src="media/logo-sm.png" alt="Rustify Logo" width="300"/>

# 🎵 Rustify

</div>

> **📢 Subscribe to the author's telegram channel for updates and more projects:** [**@vtvz_dev**](https://t.me/vtvz_dev)

> A Telegram bot that monitors your Spotify playback, detects profane lyrics and AI-generated music, integrates with AI for text analysis, and automatically skips tracks you've disliked

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build and Deploy](https://github.com/vtvz/rustify/actions/workflows/deploy.yml/badge.svg)](https://github.com/vtvz/rustify/actions/workflows/deploy.yml)

---

<div align="center">

🤖 **Try the live bot:** **[@RustifyBot](https://t.me/RustifyBot?start=telegram)**

</div>

---

Rustify is an intelligent Telegram bot that integrates with Spotify to provide real-time profanity detection, AI-generated music detection, and track management. It continuously monitors what you're listening to, analyzes lyrics for inappropriate content, detects AI-generated tracks, and automatically skips tracks you've marked with the dislike button.

## ✨ Features

### 🎯 Core Features

- **🔍 Real-time Profanity Detection** - Automatically analyzes song lyrics as you listen using advanced profanity detection algorithms
- **🤖 AI-Generated Music Detection** - Identifies AI-generated tracks using multiple detection providers and shows notifications with attribution
- **⏭️ Auto-Skip** - Instantly skips tracks you've marked with dislike
- **📊 Multi-Provider Lyrics** - Fetches lyrics from multiple sources (Musixmatch, Genius, LrcLib) for maximum coverage
- **🤖 AI-Powered Analysis** - Optional OpenAI-compatible API integration for analyzing song lyrics meaning, storyline, and content themes, plus individual word analysis
- **🌍 Multi-Language Support** - Interface available in multiple languages (profanity detection in English only)

### 🎛️ User Features

- **👍 Like/Dislike System** - Quick reactions to tracks, with automatic skipping of disliked songs
- **✨ Magic Playlist™** - Shuffled playlist of your liked songs that automatically removes tracks as you listen, ensuring no repeats
- **⏭️ Skippage™** - Skip tracks you've recently listened to (configurable time window)
- **🤖 Recommendasion™** - Get personalized track recommendations
- **📱 Interactive Keyboards** - Quick access to common actions via Telegram inline keyboards
- **🔔 Real-time Notifications** - Get notified when profane tracks are detected

### 🛡️ Admin Features

- **📊 Global Statistics** - View usage statistics across all users
- **👥 User Information** - View detailed information about users
- **🔔 New User Notifications** - Get notified when new users join
- **📢 Broadcast Messages** - Send announcements to all users

### Technology Stack

- **Language**: Rust (see [rust-toolchain.toml](rust-toolchain.toml) for exact version)
- **Telegram**: [Teloxide](https://github.com/teloxide/teloxide)
- **Spotify**: [RSpotify](https://github.com/ramsayleung/rspotify)
- **Database**: PostgreSQL with [SeaORM](https://www.sea-ql.org/SeaORM/)
- **Cache**: Redis
- **Profanity Detection**: [Rustrict](https://github.com/finnbear/rustrict) - The cornerstone of the project
- **AI**: OpenAI API (optional)
- **Metrics**: InfluxDB (optional)
- **Logging**: Grafana Loki (optional)

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📞 Support

- 🐛 [Report Issues](https://github.com/vtvz/rustify/issues/new)
- 💬 [Discussions](https://github.com/vtvz/rustify/discussions)
- 📧 Contact: [@vtvz](https://github.com/vtvz)
- 💬 Telegram: [@vtvz_me](https://t.me/vtvz_me)

---

Made with ❤️ and Rust by [@vtvz](https://github.com/vtvz)
