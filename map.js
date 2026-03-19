// 보드 생성
const board = document.getElementById("board");
const size = 7;

for (let row = 0; row < size; row++) {
  for (let col = 0; col < size; col++) {
    const div = document.createElement("div");

    const isEdge =
      row === 0 || row === size - 1 || col === 0 || col === size - 1;

    if (isEdge) {
      div.className = "tile";
      div.dataset.row = row;
      div.dataset.col = col;
    } else {
      div.className = "empty";
    }

    board.appendChild(div);
  }
}


// 경로 생성 (외곽 한 바퀴)
function createPath(size) {
  const path = [];

  for (let col = 0; col < size; col++) path.push([0, col]);
  for (let row = 1; row < size; row++) path.push([row, size - 1]);
  for (let col = size - 2; col >= 0; col--) path.push([size - 1, col]);
  for (let row = size - 2; row > 0; row--) path.push([row, 0]);

  return path;
}

const path = createPath(size);


// 게임 상태
const players = [
  { id: 1, pos: 0 },
  { id: 2, pos: 0 },
  { id: 3, pos: 0 },
  { id: 4, pos: 0 },
];

let currentPlayer = 0;
let isAnimating = false;
let turnInProgress = false; // 이번 턴 진행 중 여부 (확인 버튼 제어용)
const cellSlotMap = new Map();


// DOM
const diceEl = document.querySelector(".dice");
const rollBtn = document.getElementById("rollBtn");
const confirmBtn = document.getElementById("confirmBtn");
const balanceEl = document.getElementById("balance");

document
  .querySelectorAll(".player-marker")
  .forEach(marker => {
    marker.classList.add("pending-placement");
    const markerIdClass = Array.from(marker.classList).find(className =>
      /^player\d+$/.test(className)
    );
    if (markerIdClass) {
      marker.dataset.markerId = markerIdClass;
    }
  });

function getCellKey(row, col) {
  return `${row},${col}`;
}

function getOrCreateSlots(row, col) {
  const key = getCellKey(row, col);

  if (!cellSlotMap.has(key)) {
    cellSlotMap.set(key, [null, null, null, null]);
  }

  return cellSlotMap.get(key);
}

function releaseMarkerSlot(cell, markerId) {
  if (!cell?.classList?.contains("tile")) return;
  if (!markerId) return;

  const row = Number(cell.dataset.row);
  const col = Number(cell.dataset.col);
  const key = getCellKey(row, col);
  const slots = cellSlotMap.get(key);

  if (!slots) return;

  const slotIndex = slots.findIndex(slotMarkerId => slotMarkerId === markerId);

  if (slotIndex !== -1) {
    slots[slotIndex] = null;
  }

  if (slots.every(slotMarkerId => slotMarkerId === null)) {
    cellSlotMap.delete(key);
  }
}

function reserveMarkerSlot(row, col, markerId) {
  const slots = getOrCreateSlots(row, col);

  const existingSlotIndex = slots.findIndex(
    slotMarkerId => slotMarkerId === markerId
  );

  if (existingSlotIndex !== -1) {
    return existingSlotIndex;
  }

  const emptySlotIndex = slots.findIndex(slotMarkerId => slotMarkerId === null);

  if (emptySlotIndex !== -1) {
    slots[emptySlotIndex] = markerId;
    return emptySlotIndex;
  }

  return 0;
}

function placeMarkerInSlot(marker, row, col) {
  const markerId = marker.dataset.markerId;
  if (!markerId) return;

  const positions = [
    { x: -15, y: -15 },
    { x: 15, y: -15 },
    { x: -15, y: 15 },
    { x: 15, y: 15 },
  ];

  const slotIndex = reserveMarkerSlot(row, col, markerId);
  const pos = positions[slotIndex] ?? positions[0];

  marker.style.left = "50%";
  marker.style.top = "50%";
  marker.style.transform = `translate(-50%, -50%) translate(${pos.x}px, ${pos.y}px)`;
}


// 이동 함수
function movePlayer(playerIndex) {
  const player = players[playerIndex];
  const marker = document.querySelector(`.player${player.id}`);

  if (!marker) return;

  const [row, col] = path[player.pos];

  const cell = document.querySelector(
    `.tile[data-row="${row}"][data-col="${col}"]`
  );

  if (cell) {
    const markerId = marker.dataset.markerId;
    const previousCell = marker.parentElement;

    if (previousCell && previousCell !== cell) {
      releaseMarkerSlot(previousCell, markerId);
    }

    cell.appendChild(marker);
    marker.classList.remove("pending-placement");
    placeMarkerInSlot(marker, row, col);
  }
}


// 마커 애니메이션 이동
async function animateMove(playerIndex, steps) {
  isAnimating = true;

  const player = players[playerIndex];

  for (let i = 0; i < steps; i++) {
    player.pos = (player.pos + 1) % path.length;

    movePlayer(playerIndex);

    await new Promise(resolve => setTimeout(resolve, 300));
  }

  isAnimating = false;
}

// 콘솔용 턴 표시
function updateTurnUI() {
  const playerEls = document.querySelectorAll(".player");

  playerEls.forEach((el, idx) => {
    if (idx === currentPlayer) {
      el.classList.add("active");
    } else {
      el.classList.remove("active");
    }
  });
}

function formatMoney(amount) {
  return Number(amount).toLocaleString("ko-KR");
}

function updateBalanceUI() {
  const activePlayer = players[currentPlayer];

  if (!balanceEl || !activePlayer) return;

  const balance = activePlayer.money ?? 0;
  balanceEl.textContent = `💰 잔액: ${formatMoney(balance)}만원`;
}

function syncPlayersFromState(state) {
  state.players.forEach(playerState => {
    const player = players.find(frontPlayer => frontPlayer.id === playerState.id);

    if (!player) return;

    player.pos = playerState.position;
    player.money = playerState.money;
  });

  const nextCurrentPlayer = players.findIndex(
    player => player.id === state.current_player_id
  );

  if (nextCurrentPlayer !== -1) {
    currentPlayer = nextCurrentPlayer;
  }

  players.forEach((_, idx) => movePlayer(idx));
  updateTurnUI();
  updateBalanceUI();
}

async function fetchGameState() {
  const BASE_URL = "http://localhost:8080";
  const res = await fetch(`${BASE_URL}/api/game-state`);

  if (!res.ok) {
    throw new Error(`Failed to fetch game state: ${res.status}`);
  }

  return res.json();
}


// 주사위 (백 연결 필요)
if (rollBtn) {
  rollBtn.addEventListener("click", async (event) => {
    event.preventDefault();
    
    // 애니메이션 중이거나 턴 진행 중이면 막기
    if (isAnimating || turnInProgress) return;

    try {
      const BASE_URL = "http://localhost:8080";

      const res = await fetch(`${BASE_URL}/api/play-turn`, {
        method: "POST",
      });

      if (!res.ok) {
        throw new Error(`Backend error: ${res.status}`);
      }

      const data = await res.json();

      const {
        player_id,
        dice,
        old_position,
        new_position,
        passed_start,
        game_end,
        tile_type // 앞으로 백에서 줄 예정
      } = data;

      console.log("🎲 Dice result:", data);

      diceEl.textContent = dice;

      const playerIndex = players.findIndex(p => p.id === player_id);

      // 이동 step 계산
      const steps =
        (new_position - old_position + path.length) % path.length;

      await animateMove(playerIndex, steps);

      // 위치 동기화
      players[playerIndex].pos = new_position;

      const state = await fetchGameState();
      syncPlayersFromState(state);

      // 월급 로그 (백에서 처리됨)
      if (passed_start) {
        console.log(`💰 Player ${player_id} salary +20`);
      }

      // 게임 종료
      if (game_end) {
        alert(`Player ${player_id} wins!`);
        return;
      }

      turnInProgress = true; // 확인 버튼 활성 상태

    } catch (err) {
      console.error("❌ Roll dice error:", err);
      alert("주사위 던지기 실패: " + err.message);
    }
  });
}


// 확인버튼: 턴 넘기기
if (confirmBtn) {
  confirmBtn.addEventListener("click", async (event) => {
    event.preventDefault();
    
    if (!turnInProgress || isAnimating) return;

    try {
      resetActionButtons();

      const BASE_URL = "http://localhost:8080";

      const res = await fetch(`${BASE_URL}/api/next-turn`, {
        method: "POST",
      });

      if (!res.ok) {
        throw new Error(`Backend error: ${res.status}`);
      }

      await res.json();

      const state = await fetchGameState();
      syncPlayersFromState(state);

      turnInProgress = false;
      diceEl.textContent = "-";
    } catch (err) {
      console.error("❌ Next turn error:", err);
      alert("턴 넘기기 실패: " + err.message);
    }
  });
} else {
  console.error("❌ confirmBtn not found!");
}


// 초기화
async function initGame() {
  try {
    const BASE_URL = "http://localhost:8080";

    const res = await fetch(`${BASE_URL}/api/reset-game`, {
      method: "POST",
    });

    if (!res.ok) {
      throw new Error(`Failed to reset game: ${res.status}`);
    }

    const state = await res.json();

    turnInProgress = false;
    isAnimating = false;
    diceEl.textContent = "-";
    resetActionButtons();
    syncPlayersFromState(state);
    
  } catch (err) {
    console.error("❌ Failed to init game:", err);
    // 폴백: 기본 초기화
    players.forEach((_, idx) => movePlayer(idx));
    updateTurnUI();
    updateBalanceUI();
  }
}

initGame();


// 모든 버튼 숨기기
function resetActionButtons() {
  document.getElementById("buyBtn").style.display = "none";
  document.getElementById("payBtn").style.display = "none";
}


// 구매 버튼
const buyBtn = document.getElementById("buyBtn");

if (buyBtn) {
  buyBtn.onclick = () => {
    const result = confirm("구매하시겠습니까?");

    if (result) {
      alert("구매 완료!");
      // TODO: Rust에 구매 요청 보내기
    } else {
      console.log("구매 취소");
    }
    
  };
}

// 통행료 버튼
const payBtn = document.getElementById("payBtn");

if (payBtn) {
  payBtn.onclick = () => {
    const fee = 2000; // 임시값
    alert(`${fee}원이 출금되었습니다.`);
    // TODO: Rust에 지불 요청

    resetActionButtons(); // 행동 끝나면 버튼 숨김
  };
}


const accountBtn = document.getElementById("accountBtn");
const modal = document.getElementById("accountModal");
const closeModal = document.getElementById("closeModal");

accountBtn.onclick = () => {
  modal.classList.remove("hidden");

  loadDummyTransactions(); // 데이터 채우기
};

closeModal.onclick = () => {
  modal.classList.add("hidden");
};

function loadDummyTransactions() {
  const tbody = document.getElementById("transactionBody");

  // 기존 내용 초기화
  tbody.innerHTML = "";

  const dummyData = [
    { id: 1, type: "입금", amount: 10000, time: "12:01", target: "시작 보너스" },
    { id: 2, type: "출금", amount: 2000, time: "12:05", target: "통행료" },
    { id: 3, type: "출금", amount: 3000, time: "12:10", target: "토지 구매" },
  ];

  dummyData.forEach(tx => {
    const row = document.createElement("tr");

    row.innerHTML = `
      <td>${tx.id}</td>
      <td>${tx.type}</td>
      <td>${tx.amount}</td>
      <td>${tx.time}</td>
      <td>${tx.target}</td>
    `;

    tbody.appendChild(row);
  });
}