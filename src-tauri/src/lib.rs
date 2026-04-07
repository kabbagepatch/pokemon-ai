use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime},
};
use tauri::{Manager, State};

const BASE_URL: &str = "https://pokeapi.co/api/v2";
const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 365);

struct AppState {
    client: reqwest::Client,
    cache_root: PathBuf,
    pokemon_cache: Mutex<HashMap<String, PokemonInfo>>,
    type_cache: Mutex<HashMap<String, TypeInfo>>,
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

fn into_badge(type_info: &TypeInfo) -> TypeBadge {
    TypeBadge {
        name: type_info.name.clone(),
        display_name: type_info.display_name.clone(),
        image: type_info.image.clone(),
    }
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

#[tauri::command]
async fn get_pokemon_matchup(
    name: String,
    state: State<'_, AppState>,
) -> Result<PokemonView, String> {
    let normalized_name = name.trim().to_lowercase();
    if normalized_name.is_empty() {
        return Err("Please enter a Pokemon name or ID.".to_string());
    }

    let mut pokemon = get_pokemon_info(&state, &normalized_name).await?;

    let matchup_details = match pokemon.matchup_details.clone() {
        Some(details) => details,
        None => {
            let details = build_matchup_details(&state, &pokemon.types).await?;
            pokemon.matchup_details = Some(details.clone());
            write_pokemon_cache(&state, normalized_name.clone(), pokemon.clone())?;
            details
        }
    };

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let cache_root = app
                .path()
                .app_cache_dir()
                .map_err(|error| format!("Failed to resolve cache directory: {error}"))?;

            app.manage(AppState {
                client: reqwest::Client::new(),
                cache_root,
                pokemon_cache: Mutex::new(HashMap::new()),
                type_cache: Mutex::new(HashMap::new()),
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_pokemon_matchup])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
