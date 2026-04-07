const { invoke } = window.__TAURI__.core;

let currentPokemon = null;

const resultsBlock = document.getElementById('results');
const pokemonName = document.getElementById('pokemonName');
const pokemonImage = document.getElementById('pokemonImage');
const pokemonTypesBlock = document.getElementById('pokemonType').getElementsByClassName('pokemonTypes')[0];
const strongAgainstBlock = document.getElementById('strongAgainst').getElementsByClassName('pokemonTypes')[0];
const weakAgainstBlock = document.getElementById('weakAgainst').getElementsByClassName('pokemonTypes')[0];

const clearTypeBlock = (block) => {
  block.innerHTML = '';
};

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

const populatePokemonInfo = async (name) => {
  try {
    const pokemon = await invoke('get_pokemon_matchup', { name });
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
