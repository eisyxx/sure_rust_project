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
let diceValue = 0;
let rolled = false;
let isAnimating = false;


// DOM
const diceEl = document.querySelector(".dice");
const rollBtn = document.getElementById("rollBtn");
const confirmBtn = document.getElementById("confirmBtn");


// 이동 함수
function movePlayer(playerIndex) {
  const player = players[playerIndex];
  const marker = document.querySelector(`.player${player.id}`);

  const [row, col] = path[player.pos];

  const cell = document.querySelector(
    `.tile[data-row="${row}"][data-col="${col}"]`
  );

  if (cell) {
    cell.appendChild(marker);
    arrangeMarkersInCell(row, col);
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

  handleTileEvent(playerIndex); // 이동 완료 후 땅 상태 확인
}

// 한 칸에 여러 명이 있을 때 마커 위치 조정
function arrangeMarkersInCell(row, col) {
  const cell = document.querySelector(
    `.tile[data-row="${row}"][data-col="${col}"]`
  );

  if (!cell) return;

  const markers = cell.querySelectorAll(".player-marker");

  const positions = [
    { x: -15, y: -15 },    // 좌상
    { x: 15, y: -15 },   // 우상
    { x: -15, y: 15 },   // 좌하
    { x: 15, y: 15 },  // 우하
  ];

  markers.forEach((marker, idx) => {
    const pos = positions[idx % positions.length];

    marker.style.left = "50%";
    marker.style.top = "50%";
    marker.style.transform = `translate(-50%, -50%) translate(${pos.x}px, ${pos.y}px)`;
  });
}


// 콘솔용 턴 표시
function updateTurnUI() {
  console.log(`현재 턴: Player ${players[currentPlayer].id}`);

  const playerEls = document.querySelectorAll(".player");

  playerEls.forEach((el, idx) => {
    if (idx === currentPlayer) {
      el.classList.add("active");
    } else {
      el.classList.remove("active");
    }
  });
}


// 주사위
if (rollBtn) {
  rollBtn.onclick = async () => {
    if (rolled || isAnimating) return;

    diceValue = Math.floor(Math.random() * 6) + 1;
    diceEl.textContent = diceValue;

    rolled = true;

    await animateMove(currentPlayer, diceValue);
  };
}


// 확인버튼: 턴 넘기기
if (confirmBtn) {
  confirmBtn.onclick = async () => {
    if (!rolled || isAnimating) return;

    resetActionButtons(); // 턴 넘기기 전에 버튼 초기화

    currentPlayer = (currentPlayer + 1) % players.length;

    rolled = false;
    diceEl.textContent = "-";

    updateTurnUI();
  };
}


// 초기화
function initGame() {
  players.forEach((_, idx) => movePlayer(idx));
  updateTurnUI();
}

initGame();


// 땅 상태 (임시 랜덤)
function getTileType() {
  const types = ["buy", "toll", "event"];
  return types[Math.floor(Math.random() * types.length)];
}


// 땅 도착 시 처리
function handleTileEvent(playerIndex) {
  const type = getTileType();

  resetActionButtons(); // ⭐ 먼저 초기화

  if (type === "buy") {
    alert("구매가 가능한 토지입니다.");

    // ⭐ 구매 버튼 표시
    document.getElementById("buyBtn").style.display = "inline-block";

  } else if (type === "toll") {
    const fee = 2000;
    alert(`통행료 ${fee}원을 지불해야 합니다.`);

    // ⭐ 통행료 버튼 표시
    document.getElementById("payBtn").style.display = "inline-block";

  } else {
    alert("이벤트 칸입니다!");
  }
}


// ⭐ 모든 버튼 숨기기
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