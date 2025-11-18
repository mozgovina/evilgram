# Evilgram

A multi-bot Telegram system for managing mirror bots and broadcasting messages.

## Features

- **Multi-bot architecture** - run multiple bots from database
- **Mirror bot creation** - add new bots via existing ones  
- **Broadcast messaging** - send messages to all users across all active bots
- **Admin system** - role-based access control
- **Auto-recovery** - automatic deactivation of failed bots

## Tech Stack

- Rust, Tokio
- Teloxide (Telegram Bot API)
- MongoDB
- Anyhow (error handling)

## Prerequisites

- **MongoDB** database must be created and running
- Database name should match the one in environment variables

## Quick Start

### Environment Variables

**First run only:**
```bash
export MONGODB_URI="mongodb://localhost:27017"
export DATABASE="evilgram"            # Must be pre-created in MongoDB
export ADMIN_ID="123456789"           # Initial admin user ID  
export BOT_TOKEN="bot:token"          # Initial bot token
```

**Subsequent runs:**
```bash
export MONGODB_URI="mongodb://localhost:27017" 
export DATABASE="evilgram"            # Your pre-created database name
```

### Run
```bash
cargo run --release
```

## Admin Commands

- `/start` - Start using the bot
- `/createmirror` - Create a new mirror bot  
- `/notify` - Broadcast message to all users
- `/addadmin <user_id>` - Grant admin privileges

## Architecture

- **main.rs** - Entry point, bot loader
- **init_db.rs** - Database initialization  
- **structs.rs** - Data models (DBUser, DBBot)
- **bot.rs** - Bot logic, commands, FSM

The system automatically starts all active bots from database and provides tools for scaling through mirror bots.
