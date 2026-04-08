use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime},
};
use tauri::{Manager, State};

const BASE_URL: &str = "https://pokeapi.co/api/v2";
const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 365);
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const ANTHROPIC_MODEL: &str = "claude-sonnet-4-20250514";
const ANTHROPIC_MAX_TOKENS: u32 = 800;

struct AppState {
    client: reqwest::Client,
    cache_root: PathBuf,
    data_root: PathBuf,
    pokemon_cache: Mutex<HashMap<String, PokemonInfo>>,
    type_cache: Mutex<HashMap<String, TypeInfo>>,
    move_cache: Mutex<HashMap<String, MoveInfo>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeBadge {
    name: String,
    display_name: String,
    image: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PokemonView {
    id: u32,
    name: String,
    display_name: String,
    image: String,
    pokemon_types: Vec<TypeBadge>,
    strong_against: Vec<TypeBadge>,
    weak_against: Vec<TypeBadge>,
    matchup_details: PokemonMatchupDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PokemonInfo {
    id: u32,
    name: String,
    display_name: String,
    image: String,
    types: Vec<String>,
    #[serde(default)]
    matchup_details: Option<PokemonMatchupDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PokemonMatchupDetails {
    entries: Vec<MatchupEntry>,
    strong_against: Vec<String>,
    weak_against: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchupEntry {
    #[serde(rename = "type")]
    type_name: String,
    attack: f64,
    defense: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveInfo {
    id: u32,
    name: String,
    type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeamData {
    next_entry_id: u64,
    entries: Vec<TeamEntry>,
    current_team: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamEntry {
    id: u64,
    pokemon_name: String,
    nickname: String,
    level: u8,
    moves: Vec<Option<MoveInfo>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamPokemonDetails {
    name: String,
    display_name: String,
    image: String,
    pokemon_types: Vec<TypeBadge>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiRecommendationView {
    summary: String,
    recommendations: Vec<AiRecommendationEntryView>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiRecommendationEntryView {
    entry_id: u64,
    pokemon_name: String,
    nickname: String,
    level: u8,
    image: String,
    reason: String,
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct RecommendationPromptContext {
    opponent: PromptPokemonContext,
    roster: Vec<PromptRosterEntryContext>,
}

#[derive(Debug, Serialize)]
struct PromptPokemonContext {
    name: String,
    types: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptRosterEntryContext {
    entry_id: u64,
    is_current_team_member: bool,
    pokemon_name: String,
    nickname: String,
    level: u8,
    types: Vec<String>,
    moves: Vec<PromptMoveContext>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptMoveContext {
    name: String,
    type_name: String,
}

#[derive(Debug, Serialize)]
struct AnthropicMessageRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicRequestMessage>,
}

#[derive(Debug, Serialize)]
struct AnthropicRequestMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AiModelResponse {
    summary: String,
    recommendations: Vec<AiRecommendationModelEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiRecommendationModelEntry {
    entry_id: u64,
    reason: String,
    note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamStateView {
    entries: Vec<TeamEntry>,
    current_team_ids: Vec<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamEntryInput {
    id: Option<u64>,
    pokemon_name: String,
    nickname: String,
    level: u8,
    moves: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypeInfo {
    name: String,
    display_name: String,
    image: String,
    damage_relations: DamageRelations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DamageRelations {
    attack_double: Vec<String>,
    attack_half: Vec<String>,
    attack_none: Vec<String>,
    defense_double: Vec<String>,
    defense_half: Vec<String>,
    defense_none: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PokemonApiResponse {
    id: u32,
    name: String,
    sprites: PokemonSprites,
    types: Vec<PokemonTypeSlot>,
}

#[derive(Debug, Deserialize)]
struct PokemonSprites {
    other: PokemonOtherSprites,
}

#[derive(Debug, Deserialize)]
struct PokemonOtherSprites {
    #[serde(rename = "official-artwork")]
    official_artwork: OfficialArtwork,
}

#[derive(Debug, Deserialize)]
struct OfficialArtwork {
    front_default: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PokemonTypeSlot {
    #[serde(rename = "type")]
    type_info: NamedApiResource,
}

#[derive(Debug, Deserialize)]
struct NamedApiResource {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MoveApiResponse {
    id: u32,
    name: String,
    #[serde(rename = "type")]
    type_info: NamedApiResource,
}

#[derive(Debug, Deserialize)]
struct TypeApiResponse {
    name: String,
    sprites: TypeSprites,
    damage_relations: TypeDamageRelationsApi,
}

#[derive(Debug, Deserialize)]
struct TypeSprites {
    #[serde(rename = "generation-ix")]
    generation_ix: GenerationIxSprites,
}

#[derive(Debug, Deserialize)]
struct GenerationIxSprites {
    #[serde(rename = "scarlet-violet")]
    scarlet_violet: ScarletVioletSprites,
}

#[derive(Debug, Deserialize)]
struct ScarletVioletSprites {
    name_icon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TypeDamageRelationsApi {
    double_damage_to: Vec<NamedApiResource>,
    half_damage_to: Vec<NamedApiResource>,
    no_damage_to: Vec<NamedApiResource>,
    double_damage_from: Vec<NamedApiResource>,
    half_damage_from: Vec<NamedApiResource>,
    no_damage_from: Vec<NamedApiResource>,
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn cache_file_path(root: &Path, category: &str, key: &str) -> PathBuf {
    root.join(category).join(format!("{key}.json"))
}

fn data_file_path(root: &Path, file_name: &str) -> PathBuf {
    root.join(file_name)
}

fn is_cache_fresh(path: &Path) -> Result<bool, String> {
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("Failed to read cache metadata: {error}"))?;

    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);

    Ok(age <= CACHE_TTL)
}

fn read_file_cache<T>(root: &Path, category: &str, key: &str) -> Result<Option<T>, String>
where
    T: DeserializeOwned,
{
    let path = cache_file_path(root, category, key);
    if !path.exists() {
        return Ok(None);
    }

    if !is_cache_fresh(&path)? {
        return Ok(None);
    }

    let content =
        fs::read_to_string(&path).map_err(|error| format!("Failed to read cache file: {error}"))?;

    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("Failed to parse cache file: {error}"))
}

fn write_file_cache<T>(root: &Path, category: &str, key: &str, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    let path = cache_file_path(root, category, key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create cache directory: {error}"))?;
    }

    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Failed to serialize cache value: {error}"))?;

    fs::write(path, content).map_err(|error| format!("Failed to write cache file: {error}"))
}

fn read_data_file<T>(root: &Path, file_name: &str) -> Result<Option<T>, String>
where
    T: DeserializeOwned,
{
    let path = data_file_path(root, file_name);
    if !path.exists() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(path).map_err(|error| format!("Failed to read data file: {error}"))?;

    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("Failed to parse data file: {error}"))
}

fn write_data_file<T>(root: &Path, file_name: &str, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    fs::create_dir_all(root).map_err(|error| format!("Failed to create data directory: {error}"))?;

    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Failed to serialize data value: {error}"))?;

    fs::write(data_file_path(root, file_name), content)
        .map_err(|error| format!("Failed to write data file: {error}"))
}

async fn fetch_json<T>(client: &reqwest::Client, path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let url = format!("{BASE_URL}/{path}");
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Request failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("PokeAPI returned status {}", response.status()));
    }

    response
        .json::<T>()
        .await
        .map_err(|error| format!("Failed to parse PokeAPI response: {error}"))
}

fn read_pokemon_cache(state: &AppState, key: &str) -> Result<Option<PokemonInfo>, String> {
    let cache = state
        .pokemon_cache
        .lock()
        .map_err(|_| "Pokemon cache lock was poisoned".to_string())?;

    Ok(cache.get(key).cloned())
}

fn write_pokemon_cache(state: &AppState, key: String, value: PokemonInfo) -> Result<(), String> {
    {
        let mut cache = state
            .pokemon_cache
            .lock()
            .map_err(|_| "Pokemon cache lock was poisoned".to_string())?;

        cache.insert(key.clone(), value.clone());
    }

    write_file_cache(&state.cache_root, "pokemon", &key, &value)
}

fn read_type_cache(state: &AppState, key: &str) -> Result<Option<TypeInfo>, String> {
    let cache = state
        .type_cache
        .lock()
        .map_err(|_| "Type cache lock was poisoned".to_string())?;

    Ok(cache.get(key).cloned())
}

fn write_type_cache(state: &AppState, key: String, value: TypeInfo) -> Result<(), String> {
    {
        let mut cache = state
            .type_cache
            .lock()
            .map_err(|_| "Type cache lock was poisoned".to_string())?;

        cache.insert(key.clone(), value.clone());
    }

    write_file_cache(&state.cache_root, "types", &key, &value)
}

fn read_move_cache(state: &AppState, key: &str) -> Result<Option<MoveInfo>, String> {
    let cache = state
        .move_cache
        .lock()
        .map_err(|_| "Move cache lock was poisoned".to_string())?;

    Ok(cache.get(key).cloned())
}

fn write_move_cache(state: &AppState, key: String, value: MoveInfo) -> Result<(), String> {
    {
        let mut cache = state
            .move_cache
            .lock()
            .map_err(|_| "Move cache lock was poisoned".to_string())?;

        cache.insert(key.clone(), value.clone());
    }

    write_file_cache(&state.cache_root, "moves", &key, &value)
}

fn read_team_data(state: &AppState) -> Result<TeamData, String> {
    Ok(read_data_file::<TeamData>(&state.data_root, "team.json")?.unwrap_or(TeamData {
        next_entry_id: 1,
        entries: Vec::new(),
        current_team: Vec::new(),
    }))
}

fn write_team_data(state: &AppState, team_data: &TeamData) -> Result<(), String> {
    write_data_file(&state.data_root, "team.json", team_data)
}

async fn get_pokemon_info(state: &AppState, name: &str) -> Result<PokemonInfo, String> {
    if let Some(cached) = read_pokemon_cache(state, name)? {
        return Ok(cached);
    }

    if let Some(cached) = read_file_cache::<PokemonInfo>(&state.cache_root, "pokemon", name)? {
        write_pokemon_cache(state, name.to_string(), cached.clone())?;
        return Ok(cached);
    }

    let pokemon =
        fetch_json::<PokemonApiResponse>(&state.client, &format!("pokemon/{name}")).await?;
    let info = PokemonInfo {
        id: pokemon.id,
        name: pokemon.name.clone(),
        display_name: capitalize(&pokemon.name),
        image: pokemon
            .sprites
            .other
            .official_artwork
            .front_default
            .unwrap_or_default(),
        types: pokemon
            .types
            .into_iter()
            .map(|slot| slot.type_info.name)
            .collect(),
        matchup_details: None,
    };

    write_pokemon_cache(state, name.to_string(), info.clone())?;
    Ok(info)
}

async fn get_type_info(state: &AppState, name: &str) -> Result<TypeInfo, String> {
    if let Some(cached) = read_type_cache(state, name)? {
        return Ok(cached);
    }

    if let Some(cached) = read_file_cache::<TypeInfo>(&state.cache_root, "types", name)? {
        write_type_cache(state, name.to_string(), cached.clone())?;
        return Ok(cached);
    }

    let type_data = fetch_json::<TypeApiResponse>(&state.client, &format!("type/{name}")).await?;
    let info = TypeInfo {
        name: type_data.name.clone(),
        display_name: capitalize(&type_data.name),
        image: type_data
            .sprites
            .generation_ix
            .scarlet_violet
            .name_icon
            .unwrap_or_default(),
        damage_relations: DamageRelations {
            attack_double: type_data
                .damage_relations
                .double_damage_to
                .into_iter()
                .map(|item| item.name)
                .collect(),
            attack_half: type_data
                .damage_relations
                .half_damage_to
                .into_iter()
                .map(|item| item.name)
                .collect(),
            attack_none: type_data
                .damage_relations
                .no_damage_to
                .into_iter()
                .map(|item| item.name)
                .collect(),
            defense_double: type_data
                .damage_relations
                .double_damage_from
                .into_iter()
                .map(|item| item.name)
                .collect(),
            defense_half: type_data
                .damage_relations
                .half_damage_from
                .into_iter()
                .map(|item| item.name)
                .collect(),
            defense_none: type_data
                .damage_relations
                .no_damage_from
                .into_iter()
                .map(|item| item.name)
                .collect(),
        },
    };

    write_type_cache(state, name.to_string(), info.clone())?;
    Ok(info)
}

async fn get_move_info(state: &AppState, name: &str) -> Result<MoveInfo, String> {
    if let Some(cached) = read_move_cache(state, name)? {
        return Ok(cached);
    }

    if let Some(cached) = read_file_cache::<MoveInfo>(&state.cache_root, "moves", name)? {
        write_move_cache(state, name.to_string(), cached.clone())?;
        return Ok(cached);
    }

    let move_data = fetch_json::<MoveApiResponse>(&state.client, &format!("move/{name}")).await?;
    let info = MoveInfo {
        id: move_data.id,
        name: move_data.name,
        type_name: move_data.type_info.name,
    };

    write_move_cache(state, name.to_string(), info.clone())?;
    Ok(info)
}

fn into_badge(type_info: &TypeInfo) -> TypeBadge {
    TypeBadge {
        name: type_info.name.clone(),
        display_name: type_info.display_name.clone(),
        image: type_info.image.clone(),
    }
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

fn into_team_state_view(team_data: &TeamData) -> TeamStateView {
    TeamStateView {
        entries: team_data.entries.clone(),
        current_team_ids: team_data.current_team.clone(),
    }
}

fn load_claude_api_key() -> Result<String, String> {
    env::var("CLAUDE_API_KEY")
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Missing CLAUDE_API_KEY. Add it to your environment or local .env file.".to_string()
        })
}

fn extract_json_object(text: &str) -> Result<&str, String> {
    let start = text
        .find('{')
        .ok_or_else(|| "Claude did not return JSON.".to_string())?;
    let end = text
        .rfind('}')
        .ok_or_else(|| "Claude did not return JSON.".to_string())?;

    if end < start {
        return Err("Claude returned malformed JSON.".to_string());
    }

    Ok(&text[start..=end])
}

async fn validate_team_entry(
    state: &AppState,
    input: TeamEntryInput,
    existing_id: Option<u64>,
) -> Result<TeamEntry, String> {
    if !(1..=100).contains(&input.level) {
        return Err("Level must be between 1 and 100.".to_string());
    }

    if input.moves.len() != 4 {
        return Err("Exactly 4 move slots are required.".to_string());
    }

    let normalized_pokemon = normalize_name(&input.pokemon_name);
    if normalized_pokemon.is_empty() {
        return Err("Please enter a Pokemon name.".to_string());
    }

    let pokemon = get_pokemon_info(state, &normalized_pokemon)
        .await
        .map_err(|error| {
            format!(
                "Pokemon '{}' not found: {}",
                input.pokemon_name.trim(),
                error
            )
        })?;
    let mut validated_moves = Vec::with_capacity(4);

    for (_index, move_name) in input.moves.into_iter().enumerate() {
        let normalized_move = normalize_name(&move_name);
        if normalized_move.is_empty() {
            validated_moves.push(None);
            continue;
        }

        let move_info = get_move_info(state, &normalized_move)
            .await
            .map_err(|_error| {
                format!(
                    "Move '{}' not found",
                    move_name.trim(),
                )
            })?;
        validated_moves.push(Some(move_info));
    }

    Ok(TeamEntry {
        id: existing_id.unwrap_or_default(),
        pokemon_name: pokemon.name,
        nickname: input.nickname.trim().to_string(),
        level: input.level,
        moves: validated_moves,
    })
}

#[tauri::command]
async fn get_team_pokemon_details(
    name: String,
    state: State<'_, AppState>,
) -> Result<TeamPokemonDetails, String> {
    let normalized_name = normalize_name(&name);
    if normalized_name.is_empty() {
        return Err("Please enter a Pokemon name.".to_string());
    }

    let pokemon = get_pokemon_info(&state, &normalized_name)
        .await
        .map_err(|error| format!("Pokemon '{}' not found: {}", name.trim(), error))?;

    let mut pokemon_types = Vec::new();
    for type_name in &pokemon.types {
        let type_info = get_type_info(&state, type_name).await?;
        pokemon_types.push(into_badge(&type_info));
    }

    Ok(TeamPokemonDetails {
        name: pokemon.name,
        display_name: pokemon.display_name,
        image: pokemon.image,
        pokemon_types,
    })
}

#[derive(Debug, Clone)]
struct MatchupAccumulator {
    attack: f64,
    defense: f64,
}

impl Default for MatchupAccumulator {
    fn default() -> Self {
        Self {
            attack: 1.0,
            defense: 1.0,
        }
    }
}

fn is_neutral(value: f64) -> bool {
    (value - 1.0).abs() < f64::EPSILON
}

fn is_favorable(entry: &MatchupEntry) -> bool {
    entry.attack > 1.0 || entry.defense < 1.0
}

fn is_unfavorable(entry: &MatchupEntry) -> bool {
    entry.attack < 1.0 || entry.defense > 1.0
}

async fn build_matchup_details(
    state: &AppState,
    pokemon_types: &[String],
) -> Result<PokemonMatchupDetails, String> {
    let mut accumulators: HashMap<String, MatchupAccumulator> = HashMap::new();

    for type_name in pokemon_types {
        let type_info = get_type_info(state, type_name).await?;

        for name in &type_info.damage_relations.attack_double {
            let entry = accumulators.entry(name.clone()).or_default();
            entry.attack *= 2.0;
        }

        for name in &type_info.damage_relations.attack_half {
            let entry = accumulators.entry(name.clone()).or_default();
            entry.attack *= 0.5;
        }

        for name in &type_info.damage_relations.attack_none {
            let entry = accumulators.entry(name.clone()).or_default();
            entry.attack *= 0.0;
        }

        for name in &type_info.damage_relations.defense_double {
            let entry = accumulators.entry(name.clone()).or_default();
            entry.defense *= 2.0;
        }

        for name in &type_info.damage_relations.defense_half {
            let entry = accumulators.entry(name.clone()).or_default();
            entry.defense *= 0.5;
        }

        for name in &type_info.damage_relations.defense_none {
            let entry = accumulators.entry(name.clone()).or_default();
            entry.defense *= 0.0;
        }
    }

    let mut entries = Vec::new();
    for (type_name, accumulator) in accumulators {
        if is_neutral(accumulator.attack) && is_neutral(accumulator.defense) {
            continue;
        }

        entries.push(MatchupEntry {
            type_name,
            attack: accumulator.attack,
            defense: accumulator.defense,
        });
    }

    entries.sort_by(|left, right| left.type_name.cmp(&right.type_name));

    let mut strong_against = Vec::new();
    let mut weak_against = Vec::new();

    for entry in &entries {
        let favorable = is_favorable(entry);
        let unfavorable = is_unfavorable(entry);

        if favorable && !unfavorable {
            strong_against.push(entry.type_name.clone());
        } else if unfavorable && !favorable {
            weak_against.push(entry.type_name.clone());
        }
    }

    Ok(PokemonMatchupDetails {
        entries,
        strong_against,
        weak_against,
    })
}

async fn ensure_pokemon_matchup_details(
    state: &AppState,
    lookup_key: &str,
) -> Result<PokemonInfo, String> {
    let mut pokemon = get_pokemon_info(state, lookup_key).await?;

    if pokemon.matchup_details.is_none() {
        let details = build_matchup_details(state, &pokemon.types).await?;
        pokemon.matchup_details = Some(details);
        write_pokemon_cache(state, lookup_key.to_string(), pokemon.clone())?;
    }

    Ok(pokemon)
}

fn into_prompt_moves(moves: &[Option<MoveInfo>]) -> Vec<PromptMoveContext> {
    moves
        .iter()
        .filter_map(|move_info| move_info.as_ref())
        .map(|move_info| PromptMoveContext {
            name: move_info.name.clone(),
            type_name: move_info.type_name.clone(),
        })
        .collect()
}

async fn build_prompt_pokemon_context(
    state: &AppState,
    lookup_key: &str,
) -> Result<PromptPokemonContext, String> {
    let pokemon = ensure_pokemon_matchup_details(state, lookup_key).await?;

    Ok(PromptPokemonContext {
        name: pokemon.name,
        types: pokemon.types,
    })
}

async fn build_prompt_roster_entry_context(
    state: &AppState,
    entry: &TeamEntry,
    is_current_team_member: bool,
) -> Result<PromptRosterEntryContext, String> {
    let pokemon = ensure_pokemon_matchup_details(state, &entry.pokemon_name).await?;

    Ok(PromptRosterEntryContext {
        entry_id: entry.id,
        is_current_team_member,
        pokemon_name: pokemon.name,
        nickname: entry.nickname.clone(),
        level: entry.level,
        types: pokemon.types,
        moves: into_prompt_moves(&entry.moves),
    })
}

async fn request_claude_recommendation(
    client: &reqwest::Client,
    api_key: &str,
    prompt_context: &RecommendationPromptContext,
) -> Result<AiModelResponse, String> {
    let prompt_json = serde_json::to_string(prompt_context)
        .map_err(|error| format!("Failed to serialize AI prompt context: {error}"))?;

    let request_body = AnthropicMessageRequest {
        model: ANTHROPIC_MODEL.to_string(),
        max_tokens: ANTHROPIC_MAX_TOKENS,
        system: "You are a Pokemon battle assistant. Recommend the best available saved Pokemon to use against the provided opponent. Use only the supplied roster and opponent data. Use the roster Pokemon types and move types as primary evidence when ranking candidates. Consider level, typing, and move-type coverage. Do not invent hidden stats, abilities, items, EVs, IVs, or opponent moves. Return valid JSON only with this exact shape: {\"summary\":\"string\",\"recommendations\":[{\"entryId\":number,\"reason\":\"string\",\"note\":\"string or null\"}]}. Return at most 3 recommendations ranked best to worst. Every entryId must come from the provided roster.".to_string(),
        messages: vec![AnthropicRequestMessage {
            role: "user".to_string(),
            content: format!(
                "Here is the battle context as JSON. Choose the best available Pokemon from the roster.\n{prompt_json}"
            ),
        }],
    };
    let request_body_json = serde_json::to_string(&request_body)
        .map_err(|error| format!("Failed to serialize Claude request body: {error}"))?;

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_API_VERSION)
        .header("content-type", "application/json")
        .body(request_body_json)
        .send()
        .await
        .map_err(|error| format!("Failed to reach Claude: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let _ = response.text().await;
        return Err(format!(
            "Claude could not produce a recommendation right now (status {status}). Please try again."
        ));
    }

    let payload = response
        .json::<AnthropicMessageResponse>()
        .await
        .map_err(|error| format!("Failed to parse Claude response: {error}"))?;

    let text = payload
        .content
        .iter()
        .find(|block| block.block_type == "text")
        .and_then(|block| block.text.as_deref())
        .ok_or_else(|| "Claude returned no text content.".to_string())?;
    let json_text = extract_json_object(text)?;

    serde_json::from_str::<AiModelResponse>(json_text)
        .map_err(|error| format!("Claude returned invalid recommendation JSON: {error}"))
}

#[tauri::command]
async fn get_pokemon_matchup(
    name: String,
    state: State<'_, AppState>,
) -> Result<PokemonView, String> {
    let normalized_name = name.trim().to_lowercase();
    if normalized_name.is_empty() {
        return Err("Please enter a Pokemon name or ID.".to_string());
    }

    let pokemon = ensure_pokemon_matchup_details(&state, &normalized_name).await?;
    let matchup_details = pokemon
        .matchup_details
        .clone()
        .ok_or_else(|| "Pokemon matchup details were missing.".to_string())?;

    let mut pokemon_types = Vec::new();
    for type_name in &pokemon.types {
        let type_info = get_type_info(&state, type_name).await?;
        pokemon_types.push(into_badge(&type_info));
    }

    let mut strong_badges = Vec::new();
    for type_name in &matchup_details.strong_against {
        let type_info = get_type_info(&state, &type_name).await?;
        strong_badges.push(into_badge(&type_info));
    }

    let mut weak_badges = Vec::new();
    for type_name in &matchup_details.weak_against {
        let type_info = get_type_info(&state, &type_name).await?;
        weak_badges.push(into_badge(&type_info));
    }

    Ok(PokemonView {
        id: pokemon.id,
        name: pokemon.name,
        display_name: pokemon.display_name,
        image: pokemon.image,
        pokemon_types,
        strong_against: strong_badges,
        weak_against: weak_badges,
        matchup_details,
    })
}

#[tauri::command]
fn get_team_state(state: State<'_, AppState>) -> Result<TeamStateView, String> {
    let team_data = read_team_data(&state)?;
    Ok(into_team_state_view(&team_data))
}

#[tauri::command]
async fn get_ai_recommendation(
    opponent_name: String,
    state: State<'_, AppState>,
) -> Result<AiRecommendationView, String> {
    let normalized_name = normalize_name(&opponent_name);
    if normalized_name.is_empty() {
        return Err("Load an opponent Pokemon before asking for a recommendation.".to_string());
    }

    let team_data = read_team_data(&state)?;

    if team_data.entries.is_empty() {
        return Err("Add some Pokemon to your team or box before asking for a recommendation."
            .to_string());
    }

    let opponent = build_prompt_pokemon_context(&state, &normalized_name).await?;
    let api_key = load_claude_api_key()?;
    let mut roster = Vec::with_capacity(team_data.entries.len());

    for entry in &team_data.entries {
        let is_current_team_member = team_data.current_team.contains(&entry.id);
        roster.push(build_prompt_roster_entry_context(&state, entry, is_current_team_member).await?);
    }

    let prompt_context = RecommendationPromptContext { opponent, roster };
    let model_response =
        request_claude_recommendation(&state.client, &api_key, &prompt_context).await?;

    if model_response.recommendations.is_empty() {
        return Err("Claude did not return any recommendations.".to_string());
    }

    let entry_lookup: HashMap<u64, &TeamEntry> =
        team_data.entries.iter().map(|entry| (entry.id, entry)).collect();
    let mut seen_entry_ids = Vec::new();
    let mut recommendations = Vec::new();
    for entry in model_response.recommendations.into_iter().take(3) {
        if seen_entry_ids.contains(&entry.entry_id) {
            continue;
        }

        seen_entry_ids.push(entry.entry_id);
        let team_entry = entry_lookup.get(&entry.entry_id).ok_or_else(|| {
            format!(
                "Claude returned unknown roster entry {}.",
                entry.entry_id
            )
        })?;
        let pokemon = get_pokemon_info(&state, &team_entry.pokemon_name).await?;

        recommendations.push(AiRecommendationEntryView {
            entry_id: team_entry.id,
            pokemon_name: team_entry.pokemon_name.clone(),
            nickname: team_entry.nickname.clone(),
            level: team_entry.level,
            image: pokemon.image,
            reason: entry.reason,
            note: entry.note,
        });
    }

    if recommendations.is_empty() {
        return Err("Claude did not return any valid recommendations.".to_string());
    }

    Ok(AiRecommendationView {
        summary: model_response.summary,
        recommendations,
    })
}

#[tauri::command]
async fn save_team_entry(
    input: TeamEntryInput,
    state: State<'_, AppState>,
) -> Result<TeamStateView, String> {
    let mut team_data = read_team_data(&state)?;

    let existing_index = input
        .id
        .and_then(|entry_id| team_data.entries.iter().position(|entry| entry.id == entry_id));

    let existing_id = input.id;
    let mut validated_entry = validate_team_entry(&state, input, existing_id).await?;

    match existing_index {
        Some(index) => {
            validated_entry.id = team_data.entries[index].id;
            team_data.entries[index] = validated_entry;
        }
        None => {
            validated_entry.id = team_data.next_entry_id;
            team_data.next_entry_id += 1;
            team_data.entries.push(validated_entry);
        }
    }

    write_team_data(&state, &team_data)?;
    Ok(into_team_state_view(&team_data))
}

#[tauri::command]
fn delete_team_entry(entry_id: u64, state: State<'_, AppState>) -> Result<TeamStateView, String> {
    let mut team_data = read_team_data(&state)?;
    let initial_len = team_data.entries.len();
    team_data.entries.retain(|entry| entry.id != entry_id);

    if team_data.entries.len() == initial_len {
        return Err("Team entry not found.".to_string());
    }

    team_data.current_team.retain(|id| *id != entry_id);
    write_team_data(&state, &team_data)?;
    Ok(into_team_state_view(&team_data))
}

#[tauri::command]
fn set_team_member(
    entry_id: u64,
    selected: bool,
    state: State<'_, AppState>,
) -> Result<TeamStateView, String> {
    let mut team_data = read_team_data(&state)?;
    if !team_data.entries.iter().any(|entry| entry.id == entry_id) {
        return Err("Team entry not found.".to_string());
    }

    if selected {
        if !team_data.current_team.contains(&entry_id) {
            if team_data.current_team.len() >= 6 {
                return Err("Your current team can only have 6 Pokemon.".to_string());
            }

            team_data.current_team.push(entry_id);
        }
    } else {
        team_data.current_team.retain(|id| *id != entry_id);
    }

    write_team_data(&state, &team_data)?;
    Ok(into_team_state_view(&team_data))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            dotenvy::dotenv().ok();

            let cache_root = app
                .path()
                .app_cache_dir()
                .map_err(|error| format!("Failed to resolve cache directory: {error}"))?;
            let data_root = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

            app.manage(AppState {
                client: reqwest::Client::new(),
                cache_root,
                data_root,
                pokemon_cache: Mutex::new(HashMap::new()),
                type_cache: Mutex::new(HashMap::new()),
                move_cache: Mutex::new(HashMap::new()),
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_pokemon_matchup,
            get_ai_recommendation,
            get_team_state,
            get_team_pokemon_details,
            save_team_entry,
            delete_team_entry,
            set_team_member
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
