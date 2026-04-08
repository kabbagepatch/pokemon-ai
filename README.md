# Pokemon AI Assistant

Skip the type charts and complex calculations and get the matchup info you need for an opponent Pokemon quickly.

The app supports both:
- web lookup mode with direct PokeAPI fetches and lightweight in-memory caching
- Tauri desktop mode with filesystem-backed caching, saved team storage, and AI recommendations

## Features

- Look up a Pokemon by name or ID
- View its types, strengths, and weaknesses
- Save your own Pokemon team with nicknames, levels, and moves
- Ask AI for a recommendation on which saved Pokemon to use against the current opponent
- Pokemon, type, and move data cached on desktop filesystem for faster repeat use

## Screenshots

### Lookup

<img height="500" alt="image" src="https://github.com/user-attachments/assets/df18e4ab-1517-454b-a159-24f2b576d41a" />
<img height="500" alt="image" src="https://github.com/user-attachments/assets/c3a290b2-a428-4d90-85f9-d551040c5aa8" />

### Teams

<img height="500" alt="image" src="https://github.com/user-attachments/assets/225029de-a875-4be4-9f28-d7797b6bfd8f" />
<img height="500" alt="image" src="https://github.com/user-attachments/assets/452c3967-5917-4bfd-b325-0eaff25a9448" />

### AI Assistant



## AI Recommendations

In the desktop app, the lookup page includes an AI recommendation button that suggests the best picks from your saved roster for the currently viewed opponent.

The Rust backend:
- builds the recommendation prompt from your saved team and box data
- uses cached Pokemon and move data as the factual input
- sends the request to Claude and returns structured recommendations for the UI

## Local Setup

Install dependencies and run the Tauri app as usual.

For desktop AI recommendations, create a local `.env` file in the project root with:

```env
CLAUDE_API_KEY=your_anthropic_key_here
```

Notes:
- `.env` is ignored by git and should stay local
- the AI feature is desktop-only so the Claude key stays on the Rust side
- if the key is missing, the AI recommendation action will fail with a friendly error
