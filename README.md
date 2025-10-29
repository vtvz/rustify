# 🎵 Rustify

> **📢 Subscribe to the author's telegram channel for updates and more projects:** [**@vtvz_dev**](https://t.me/vtvz_dev)

> A Telegram bot that monitors your Spotify playback, detects profane lyrics, integrates with AI for text analysis, and automatically skips tracks you've disliked

[![Rust](https://img.shields.io/badge/rust-1.83.0%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Rustify is an intelligent Telegram bot that integrates with Spotify to provide real-time profanity filtering and track management. It continuously monitors what you're listening to, analyzes lyrics for inappropriate content, and automatically skips tracks you've marked with the dislike button.

## ✨ Features

### 🎯 Core Features

- **🔍 Real-time Profanity Detection** - Automatically analyzes song lyrics as you listen using advanced profanity detection algorithms
- **⏭️ Auto-Skip** - Instantly skips tracks you've marked with dislike (can be toggled)
- **📊 Multi-Provider Lyrics** - Fetches lyrics from multiple sources (Musixmatch, Genius, LrcLib) for maximum coverage
- **🤖 AI-Powered Analysis** - Optional OpenAI integration for analyzing song lyrics meaning, storyline, and content themes, plus individual word analysis
- **🌍 Multi-Language Support** - Interface available in multiple languages with localized profanity detection

### 🎛️ User Features

- **👍 Like/Dislike System** - Quick reactions to tracks, with automatic skipping of disliked songs
- **✨ Magic Playlist™** - Shuffled playlist of your liked songs that automatically removes tracks as you listen, ensuring no repeats
- **⏭️ Skippage** - Skip tracks you've recently listened to (configurable time window)
- **🤖 Recommendasion™** - Get personalized track recommendations
- **📱 Interactive Keyboards** - Quick access to common actions via Telegram inline keyboards
- **🔔 Real-time Notifications** - Get notified when profane tracks are detected

### 🛡️ Admin Features

- **📊 Global Statistics** - View usage statistics across all users
- **📢 Broadcast Messages** - Send announcements to all users

## 🏗️ Architecture

Rustify uses a multi-worker architecture for optimal performance:

- **Bot Worker** - Handles Telegram interactions and user commands
- **Track Check Worker** - Polls Spotify API every 3 seconds to monitor playback
- **Queue Worker** - Processes background tasks (lyrics fetching, profanity analysis)

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

This project is licensed under the MIT License - see the LICENSE file for details.

## 📞 Support

- 🐛 [Report Issues](https://github.com/vtvz/rustify/issues/new)
- 💬 [Discussions](https://github.com/vtvz/rustify/discussions)
- 📧 Contact: [@vtvz](https://github.com/vtvz)

---

Made with ❤️ and Rust by [@vtvz](https://github.com/vtvz)
