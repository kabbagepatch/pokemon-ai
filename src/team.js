const invoke = window.__TAURI__?.core?.invoke;
const isDesktopRuntime = typeof invoke === 'function';
const BASE_URL = 'https://pokeapi.co/api/v2';

const currentTeamTiles = document.getElementById('currentTeamTiles');
const currentTeamDetail = document.getElementById('currentTeamDetail');
const boxTiles = document.getElementById('boxTiles');
const boxDetail = document.getElementById('boxDetail');
const currentTeamCount = document.getElementById('currentTeamCount');
const formTitle = document.getElementById('formTitle');
const teamForm = document.getElementById('teamForm');
const entryIdInput = document.getElementById('entryId');
const pokemonNameInput = document.getElementById('pokemonNameInput');
const nicknameInput = document.getElementById('nicknameInput');
const levelInput = document.getElementById('levelInput');
const moveInputs = [
  document.getElementById('move1Input'),
  document.getElementById('move2Input'),
  document.getElementById('move3Input'),
  document.getElementById('move4Input'),
];
const saveEntryButton = document.getElementById('saveEntryButton');
const cancelEditButton = document.getElementById('cancelEditButton');

document.querySelectorAll('[data-desktop-only]').forEach((element) => {
  element.hidden = !isDesktopRuntime;
});

let teamState = {
  entries: [],
  currentTeamIds: [],
};

let selectedCurrentTeamId = null;
let selectedBoxId = null;

const pokemonDetailsCache = new Map();
const webPokemonCache = new Map();
const webTypeCache = new Map();

const createInfoMessage = (message) => {
  const messageBlock = document.createElement('p');
  messageBlock.className = 'empty-state';
  messageBlock.textContent = message;
  return messageBlock;
};

const createIconButton = (label, title, onClick, className = '') => {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = `icon-button${className ? ` ${className}` : ''}`;
  button.textContent = label;
  button.title = title;
  button.setAttribute('aria-label', title);
  button.addEventListener('click', onClick);
  return button;
};

const formatDisplayName = (value) => value
  .split('-')
  .filter(Boolean)
  .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
  .join(' ');

const normalizeName = (value) => value.trim().toLowerCase().split(/\s+/).join('-');

const getTypeImageUrl = (typeName) => {
  const typeId = {
    normal: 1,
    fighting: 2,
    flying: 3,
    poison: 4,
    ground: 5,
    rock: 6,
    bug: 7,
    ghost: 8,
    steel: 9,
    fire: 10,
    water: 11,
    grass: 12,
    electric: 13,
    psychic: 14,
    ice: 15,
    dragon: 16,
    dark: 17,
    fairy: 18,
  }[typeName];

  return typeId
    ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/types/generation-ix/scarlet-violet/${typeId}.png`
    : '';
};

const fetchJson = async (path) => {
  const response = await fetch(`${BASE_URL}/${path}`);
  if (!response.ok) {
    throw new Error(`PokeAPI returned status ${response.status}`);
  }

  return response.json();
};

const getTypeInfoWeb = async (name) => {
  const normalizedName = normalizeName(name);
  if (webTypeCache.has(normalizedName)) {
    return webTypeCache.get(normalizedName);
  }

  const typeData = await fetchJson(`type/${normalizedName}`);
  const typeInfo = {
    name: typeData.name,
    displayName: formatDisplayName(typeData.name),
    image: typeData.sprites['generation-ix']['scarlet-violet'].name_icon ?? '',
  };
  webTypeCache.set(normalizedName, typeInfo);
  return typeInfo;
};

const getPokemonDetailsWeb = async (name) => {
  const normalizedName = normalizeName(name);
  if (webPokemonCache.has(normalizedName)) {
    return webPokemonCache.get(normalizedName);
  }

  const pokemon = await fetchJson(`pokemon/${normalizedName}`);
  const pokemonTypes = [];
  for (const slot of pokemon.types) {
    const typeInfo = await getTypeInfoWeb(slot.type.name);
    pokemonTypes.push(typeInfo);
  }

  const details = {
    name: pokemon.name,
    displayName: formatDisplayName(pokemon.name),
    image: pokemon.sprites.other['official-artwork'].front_default ?? '',
    pokemonTypes,
  };

  webPokemonCache.set(normalizedName, details);
  return details;
};

const getPokemonDetails = async (pokemonName) => {
  const normalizedName = normalizeName(pokemonName);
  if (pokemonDetailsCache.has(normalizedName)) {
    return pokemonDetailsCache.get(normalizedName);
  }

  const detailPromise = isDesktopRuntime
    ? invoke('get_team_pokemon_details', { name: normalizedName })
    : getPokemonDetailsWeb(normalizedName);

  pokemonDetailsCache.set(normalizedName, detailPromise);
  return detailPromise;
};

const populateEntries = async (entries) => {
  await Promise.all(entries.map(async (entry) => {
    entry.details = await getPokemonDetails(entry.pokemonName);
  }));
};

const getCurrentTeamEntries = () => teamState.currentTeamIds
  .map((teamId) => teamState.entries.find((entry) => entry.id === teamId))
  .filter(Boolean);

const getBoxEntries = () => teamState.entries
  .filter((entry) => !teamState.currentTeamIds.includes(entry.id));

const createTile = (entry, selectedId, onSelect) => {
  const tile = document.createElement('button');
  tile.type = 'button';
  tile.className = `pokemon-tile${selectedId === entry.id ? ' selected' : ''}`;
  tile.title = entry.nickname || entry.details?.displayName || formatDisplayName(entry.pokemonName);

  const image = document.createElement('img');
  image.src = entry.details?.image ?? '';
  image.alt = entry.details?.displayName ?? formatDisplayName(entry.pokemonName);
  image.className = 'pokemon-tile-image';
  tile.appendChild(image);

  tile.addEventListener('click', () => {
    onSelect(selectedId === entry.id ? null : entry.id);
  });

  return tile;
};

const createTypeRow = (pokemonTypes) => {
  const typeRow = document.createElement('div');
  typeRow.className = 'detail-type-row';

  (pokemonTypes ?? []).forEach((typeInfo) => {
    const image = document.createElement('img');
    image.src = typeInfo.image;
    image.alt = typeInfo.displayName;
    image.className = 'detail-type-image';
    typeRow.appendChild(image);
  });

  return typeRow;
};

const createMoveCard = (move) => {
  const moveCard = document.createElement('div');
  moveCard.className = 'move-card';

  const moveTitleRow = document.createElement('div');
  moveTitleRow.className = 'move-title-row';

  const moveName = document.createElement('span');
  moveName.className = 'move-name';
  moveName.textContent = formatDisplayName(move.name);
  moveTitleRow.appendChild(moveName);

  const moveType = document.createElement('img');
  moveType.src = getTypeImageUrl(move.typeName);
  moveType.alt = formatDisplayName(move.typeName);
  moveType.className = 'move-type-image';
  moveTitleRow.appendChild(moveType);

  moveCard.appendChild(moveTitleRow);
  return moveCard;
};

const createMoveGrid = (moves) => {
  const moveGrid = document.createElement('div');
  moveGrid.className = 'move-grid';

  const activeMoves = moves.filter(Boolean);
  if (activeMoves.length === 0) {
    moveGrid.appendChild(createInfoMessage('No saved moves'));
    return moveGrid;
  }

  activeMoves.forEach((move) => {
    moveGrid.appendChild(createMoveCard(move));
  });

  return moveGrid;
};

const createDetailCard = (entry) => {
  const card = document.createElement('article');
  card.className = 'team-detail-card';

  const info = document.createElement('div');
  info.className = 'detail-info';

  const topRow = document.createElement('div');
  topRow.className = 'detail-top-row';

  const nickname = document.createElement('h3');
  nickname.textContent = entry.nickname || entry.details?.displayName || formatDisplayName(entry.pokemonName);
  topRow.appendChild(nickname);

  const topActions = document.createElement('div');
  topActions.className = 'detail-top-actions';

  const isInTeam = teamState.currentTeamIds.includes(entry.id);
  topActions.appendChild(createIconButton(
    isInTeam ? '−' : '+',
    isInTeam ? 'Remove from team' : 'Add to team',
    async () => {
      try {
        teamState = await invoke('set_team_member', {
          entryId: entry.id,
          selected: !isInTeam,
        });
        await populateEntries(teamState.entries);
        if (isInTeam) {
          selectedCurrentTeamId = null;
        } else {
          selectedBoxId = null;
        }
        render();
      } catch (error) {
        alert(String(error));
      }
    },
    isInTeam ? 'remove' : 'add',
  ));

  topActions.appendChild(createIconButton(
    '✎',
    'Edit',
    () => fillForm(entry),
    'edit',
  ));

  topActions.appendChild(createIconButton(
    '×',
    'Delete',
    async () => {
      try {
        teamState = await invoke('delete_team_entry', { entryId: entry.id });
        if (entryIdInput.value === String(entry.id)) {
          resetForm();
        }
        selectedBoxId = selectedBoxId === entry.id ? null : selectedBoxId;
        selectedCurrentTeamId = selectedCurrentTeamId === entry.id ? null : selectedCurrentTeamId;
        await populateEntries(teamState.entries);
        render();
      } catch (error) {
        alert(String(error));
      }
    },
    'delete',
  ));

  topRow.appendChild(topActions);

  info.appendChild(topRow);

  const speciesRow = document.createElement('div');
  speciesRow.className = 'detail-species-row';

  const species = document.createElement('span');
  species.className = 'card-copy';
  species.textContent = `${entry.details?.displayName || formatDisplayName(entry.pokemonName)} · Lv. ${entry.level}`;
  speciesRow.appendChild(species);

  speciesRow.appendChild(createTypeRow(entry.details?.pokemonTypes));
  info.appendChild(speciesRow);
  info.appendChild(createMoveGrid(entry.moves));

  card.appendChild(info);
  return card;
};

const renderTileSection = (entries, tileContainer, detailContainer, selectedId, onSelect) => {
  tileContainer.innerHTML = '';

  if (entries.length === 0) {
    tileContainer.className = 'team-tile-grid empty-state';
    tileContainer.textContent = 'Empty.';
    detailContainer.hidden = true;
    detailContainer.innerHTML = '';
    return;
  }

  tileContainer.className = 'team-tile-grid';
  entries.forEach((entry) => {
    tileContainer.appendChild(createTile(entry, selectedId, onSelect));
  });

  const selectedEntry = entries.find((entry) => entry.id === selectedId);
  if (!selectedEntry) {
    detailContainer.hidden = true;
    detailContainer.innerHTML = '';
    return;
  }

  detailContainer.hidden = false;
  detailContainer.innerHTML = '';
  detailContainer.appendChild(createDetailCard(selectedEntry));
};

const resetForm = () => {
  teamForm.reset();
  entryIdInput.value = '';
  levelInput.value = '50';
  formTitle.textContent = 'Add Pokemon';
  saveEntryButton.textContent = 'Save';
};

const fillForm = (entry) => {
  entryIdInput.value = String(entry.id);
  pokemonNameInput.value = entry.details?.displayName || formatDisplayName(entry.pokemonName);
  nicknameInput.value = entry.nickname;
  levelInput.value = String(entry.level);

  moveInputs.forEach((input, index) => {
    input.value = formatDisplayName(entry.moves[index]?.name ?? '');
  });

  formTitle.textContent = `Edit ${entry.nickname || entry.details?.displayName || formatDisplayName(entry.pokemonName)}`;
  saveEntryButton.textContent = 'Update Pokemon';
};

const syncSelectedIds = () => {
  const currentTeamEntries = getCurrentTeamEntries();
  const boxEntries = getBoxEntries();
  currentTeamCount.textContent = `${currentTeamEntries.length} / 6`;

  if (!currentTeamEntries.some((entry) => entry.id === selectedCurrentTeamId)) {
    selectedCurrentTeamId = null;
  }

  if (!boxEntries.some((entry) => entry.id === selectedBoxId)) {
    selectedBoxId = null;
  }
};

const render = () => {
  const currentTeamEntries = getCurrentTeamEntries();
  const boxEntries = getBoxEntries();
  syncSelectedIds();

  renderTileSection(
    currentTeamEntries,
    currentTeamTiles,
    currentTeamDetail,
    selectedCurrentTeamId,
    (entryId) => {
      selectedCurrentTeamId = entryId;
      render();
    },
  );

  renderTileSection(
    boxEntries,
    boxTiles,
    boxDetail,
    selectedBoxId,
    (entryId) => {
      selectedBoxId = entryId;
      render();
    },
  );
};

const setDesktopAvailability = (enabled) => {
  Array.from(teamForm.elements).forEach((element) => {
    element.disabled = !enabled;
  });
};

const loadTeamState = async () => {
  if (!isDesktopRuntime) {
    setDesktopAvailability(false);
    currentTeamTiles.innerHTML = '';
    currentTeamTiles.appendChild(createInfoMessage('Open the desktop app to manage your saved team.'));
    boxTiles.innerHTML = '';
    boxTiles.appendChild(createInfoMessage('Roster persistence is unavailable in the web version.'));
    currentTeamDetail.hidden = true;
    boxDetail.hidden = true;
    return;
  }

  teamState = await invoke('get_team_state');
  await populateEntries(teamState.entries);
  selectedCurrentTeamId = null;
  selectedBoxId = null;
  render();
};

teamForm.addEventListener('submit', async (event) => {
  event.preventDefault();

  if (!isDesktopRuntime) {
    return;
  }

  const input = {
    id: entryIdInput.value ? Number.parseInt(entryIdInput.value, 10) : null,
    pokemonName: pokemonNameInput.value,
    nickname: nicknameInput.value,
    level: Number.parseInt(levelInput.value, 10),
    moves: moveInputs.map((inputElement) => inputElement.value),
  };

  try {
    teamState = await invoke('save_team_entry', { input });
    await populateEntries(teamState.entries);
    resetForm();
    selectedBoxId = null;
    render();
  } catch (error) {
    alert(String(error));
  }
});

cancelEditButton.addEventListener('click', () => {
  resetForm();
});

resetForm();
loadTeamState().catch((error) => {
  console.error(error);
});
