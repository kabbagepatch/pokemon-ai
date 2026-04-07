const invoke = window.__TAURI__?.core?.invoke;
const isDesktopRuntime = typeof invoke === 'function';
const BASE_URL = 'https://pokeapi.co/api/v2';

let currentPokemon = null;

const pokemonCache = new Map();
const typeCache = new Map();

const resultsBlock = document.getElementById('results');
const pokemonName = document.getElementById('pokemonName');
const pokemonImage = document.getElementById('pokemonImage');
const pokemonTypesBlock = document.getElementById('pokemonType').getElementsByClassName('pokemonTypes')[0];
const strongAgainstBlock = document.getElementById('strongAgainst').getElementsByClassName('pokemonTypes')[0];
const weakAgainstBlock = document.getElementById('weakAgainst').getElementsByClassName('pokemonTypes')[0];

const clearTypeBlock = (block) => {
  block.innerHTML = '';
};

const capitalize = (value) => {
  if (!value) {
    return '';
  }

  return value.charAt(0).toUpperCase() + value.slice(1);
};

const normalizePokemonName = (name) => name.trim().toLowerCase();

const appendTypeImages = (block, types) => {
  clearTypeBlock(block);

  if (types.length === 0) {
    block.innerHTML = '<p>None</p>';
    return;
  }

  types.forEach((type) => {
    const image = document.createElement('img');
    image.src = type.image;
    image.alt = type.displayName;
    image.className = 'type-image';
    block.appendChild(image);
  });
};

const fetchJson = async (path) => {
  const response = await fetch(`${BASE_URL}/${path}`);

  if (!response.ok) {
    throw new Error(`PokeAPI returned status ${response.status}`);
  }

  return response.json();
};

const toTypeBadge = (typeInfo) => ({
  name: typeInfo.name,
  displayName: typeInfo.displayName,
  image: typeInfo.image,
});

const getPokemonInfoWeb = async (name) => {
  const normalizedName = normalizePokemonName(name);

  if (!normalizedName) {
    throw new Error('Please enter a Pokemon name or ID.');
  }

  if (pokemonCache.has(normalizedName)) {
    return pokemonCache.get(normalizedName);
  }

  const pokemon = await fetchJson(`pokemon/${normalizedName}`);
  const info = {
    id: pokemon.id,
    name: pokemon.name,
    displayName: capitalize(pokemon.name),
    image: pokemon.sprites.other['official-artwork'].front_default ?? '',
    types: pokemon.types.map((slot) => slot.type.name),
  };

  pokemonCache.set(normalizedName, info);
  return info;
};

const getTypeInfoWeb = async (name) => {
  const normalizedName = name.trim().toLowerCase();

  if (typeCache.has(normalizedName)) {
    return typeCache.get(normalizedName);
  }

  const typeData = await fetchJson(`type/${normalizedName}`);
  const info = {
    name: typeData.name,
    displayName: capitalize(typeData.name),
    image: typeData.sprites['generation-ix']['scarlet-violet'].name_icon ?? '',
    damageRelations: {
      attackDouble: typeData.damage_relations.double_damage_to.map((item) => item.name),
      attackHalf: typeData.damage_relations.half_damage_to.map((item) => item.name),
      attackNone: typeData.damage_relations.no_damage_to.map((item) => item.name),
      defenseDouble: typeData.damage_relations.double_damage_from.map((item) => item.name),
      defenseHalf: typeData.damage_relations.half_damage_from.map((item) => item.name),
      defenseNone: typeData.damage_relations.no_damage_from.map((item) => item.name),
    },
  };

  typeCache.set(normalizedName, info);
  return info;
};

const getPokemonMatchupWeb = async (name) => {
  const normalizedName = normalizePokemonName(name);

  if (!normalizedName) {
    throw new Error('Please enter a Pokemon name or ID.');
  }

  const pokemon = await getPokemonInfoWeb(normalizedName);
  const pokemonTypes = [];
  const strongAgainst = new Set();
  const weakAgainst = new Set();

  for (const typeName of pokemon.types) {
    const typeInfo = await getTypeInfoWeb(typeName);
    pokemonTypes.push(toTypeBadge(typeInfo));

    typeInfo.damageRelations.attackDouble.forEach((matchupName) => strongAgainst.add(matchupName));
    typeInfo.damageRelations.attackHalf.forEach((matchupName) => weakAgainst.add(matchupName));
    typeInfo.damageRelations.attackNone.forEach((matchupName) => weakAgainst.add(matchupName));
    typeInfo.damageRelations.defenseDouble.forEach((matchupName) => weakAgainst.add(matchupName));
    typeInfo.damageRelations.defenseHalf.forEach((matchupName) => strongAgainst.add(matchupName));
    typeInfo.damageRelations.defenseNone.forEach((matchupName) => strongAgainst.add(matchupName));
  }

  const overlappingTypes = [...strongAgainst].filter((typeName) => weakAgainst.has(typeName));
  overlappingTypes.forEach((typeName) => {
    strongAgainst.delete(typeName);
    weakAgainst.delete(typeName);
  });

  const strongBadges = [];
  for (const typeName of [...strongAgainst].sort()) {
    const typeInfo = await getTypeInfoWeb(typeName);
    strongBadges.push(toTypeBadge(typeInfo));
  }

  const weakBadges = [];
  for (const typeName of [...weakAgainst].sort()) {
    const typeInfo = await getTypeInfoWeb(typeName);
    weakBadges.push(toTypeBadge(typeInfo));
  }

  return {
    id: pokemon.id,
    name: pokemon.name,
    displayName: pokemon.displayName,
    image: pokemon.image,
    pokemonTypes,
    strongAgainst: strongBadges,
    weakAgainst: weakBadges,
  };
};

const getPokemonMatchupDesktop = (name) => invoke('get_pokemon_matchup', { name });

const getPokemonMatchup = (name) => (
  isDesktopRuntime ? getPokemonMatchupDesktop(name) : getPokemonMatchupWeb(name)
);

const populatePokemonInfo = async (name) => {
  try {
    const pokemon = await getPokemonMatchup(name);
    currentPokemon = pokemon;
    resultsBlock.hidden = false;

    pokemonName.textContent = `${pokemon.displayName.split('-')[0]} #${pokemon.id}`;
    pokemonImage.src = pokemon.image;
    pokemonImage.alt = pokemon.displayName;

    appendTypeImages(pokemonTypesBlock, pokemon.pokemonTypes);
    appendTypeImages(strongAgainstBlock, pokemon.strongAgainst);
    appendTypeImages(weakAgainstBlock, pokemon.weakAgainst);
  } catch (error) {
    console.error(error);
    alert(`Pokemon ${name} not found. Check the spelling or try using the Pokemon ID.`);
  }
};

populatePokemonInfo((Math.floor(Math.random() * 900)).toString());

const searchForm = document.getElementById('searchForm');
searchForm.addEventListener('submit', (event) => {
  event.preventDefault();
  const formData = new FormData(searchForm);
  const pokemon = formData.get('pokemon');
  if (!pokemon) {
    return;
  }

  populatePokemonInfo(pokemon.trim());
});

document.getElementById('prevButton').addEventListener('click', (event) => {
  event.preventDefault();

  if (!currentPokemon) {
    return;
  }

  const id = Number.parseInt(currentPokemon.id, 10);
  if (id <= 1) {
    return;
  }

  populatePokemonInfo((id - 1).toString());
});

document.getElementById('nextButton').addEventListener('click', (event) => {
  event.preventDefault();

  if (!currentPokemon) {
    return;
  }

  const id = Number.parseInt(currentPokemon.id, 10);
  populatePokemonInfo((id + 1).toString());
});
