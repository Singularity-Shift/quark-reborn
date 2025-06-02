# Quark Reborn

A next-generation Telegram bot built in Rust, leveraging [teloxide](https://github.com/teloxide/teloxide) for Telegram integration and [open-ai-rust-responses-by-sshift](https://github.com/Singularity-Shift/openai-rust-responses-sshift) for advanced OpenAI API features.

## Features
- Written in Rust for safety and performance
- Telegram bot framework via teloxide
- OpenAI GPT, image generation, and advanced tools via open-ai-rust-responses-by-sshift
- Async, production-ready architecture
- Extensible for future Web3 and AI integrations

## Getting Started

### Prerequisites
- Rust (latest stable recommended)
- A Telegram bot token ([how to get one](https://core.telegram.org/bots#6-botfather))
- An OpenAI API key ([get one here](https://platform.openai.com/account/api-keys))

### Setup
1. **Clone the repository:**
   ```sh
   git clone https://github.com/Singularity-Shift/quark-reborn.git
   cd quark-reborn
   ```
2. **Install dependencies:**
   ```sh
   cargo build
   ```
3. **Configure environment:**
   - Copy `env.sample.txt` to `.env` and fill in your credentials:
     ```sh
     cp env.sample.txt .env
     # Edit .env with your TELOXIDE_TOKEN and OPENAI_API_KEY
     ```
4. **Run the bot:**
   ```sh
   cargo run
   ```

## License

This project is licensed under the [Apache License 2.0](LICENSE).

---

**Powered by:**
- [teloxide](https://github.com/teloxide/teloxide)
- [open-ai-rust-responses-by-sshift](https://github.com/Singularity-Shift/openai-rust-responses-sshift) 