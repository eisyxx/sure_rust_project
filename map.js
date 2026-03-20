const board = document.getElementById("board");
const diceEl = document.querySelector(".dice");
const rollBtn = document.getElementById("rollBtn");
const confirmBtn = document.getElementById("confirmBtn");
const buyBtn = document.getElementById("buyBtn");
const payBtn = document.getElementById("payBtn");
const balanceEl = document.getElementById("balance");
const accountBtn = document.getElementById("accountBtn");
const modal = document.getElementById("accountModal");
const closeModal = document.getElementById("closeModal");
const transactionBody = document.getElementById("transactionBody");

const size = 7;
const players = [];
const path = createPath(size);

let currentPlayerId = null;
let gameFinished = false;
let isAnimating = false;
let pendingDecideResult = null;
let pendingTurnResult = null;

buildBoard();
hideUnusedButtons();
bindEvents();
initGame();

function buildBoard() {
  for (let row = 0; row < size; row += 1) {
    for (let col = 0; col < size; col += 1) {
      const div = document.createElement("div");
      const isEdge = row === 0 || row === size - 1 || col === 0 || col === size - 1;

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
}

function createPath(boardSize) {
  const boardPath = [];

  for (let col = 0; col < boardSize; col += 1) boardPath.push([0, col]);
  for (let row = 1; row < boardSize; row += 1) boardPath.push([row, boardSize - 1]);
  for (let col = boardSize - 2; col >= 0; col -= 1) boardPath.push([boardSize - 1, col]);
  for (let row = boardSize - 2; row > 0; row -= 1) boardPath.push([row, 0]);

  return boardPath;
}

function hideUnusedButtons() {
  buyBtn.style.display = "";
  buyBtn.disabled = true;
  payBtn.style.display = "none";
  confirmBtn.style.display = "";
  confirmBtn.disabled = true;
}

function bindEvents() {
  rollBtn.onclick = async () => {
    if (isAnimating || gameFinished) {
      return;
    }

    rollBtn.disabled = true;
    let waitingForDecision = false;

    try {
      const response = await fetch("/api/turn", {
        method: "POST",
      });

      if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "턴 실행 실패");
      }

      const result = await response.json();
      diceEl.textContent = String(result.dice);

      await animateTurn(result.player_id, result.dice);

      if (result.action_type === "can_buy") {
        applyState(result);
        waitingForDecision = true;
        showBuyDecision(result.action_amount);
      } else {
        pendingTurnResult = result;
        confirmBtn.disabled = false;
        showTurnMessage(result);
      }
    } catch (error) {
      alert(error.message || "턴 실행 중 오류가 발생했습니다.");
    } finally {
      if (!waitingForDecision && !gameFinished) {
        rollBtn.disabled = false;
      }
    }
  };

  buyBtn.onclick = async () => {
    buyBtn.disabled = true;

    try {
      const response = await fetch("/api/decide", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ will_buy: true }),
      });

      if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "결정 처리 실패");
      }

      pendingDecideResult = await response.json();

      const updatedCurrentPlayer = pendingDecideResult.players?.find(
        (player) => player.id === currentPlayerId,
      );

      if (updatedCurrentPlayer) {
        const localCurrentPlayer = players.find((player) => player.id === currentPlayerId);

        if (localCurrentPlayer) {
          localCurrentPlayer.money = updatedCurrentPlayer.money;
          updateBalance();
        }
      }

      alert("구매에 성공했습니다!");
      buyBtn.disabled = true;
      confirmBtn.disabled = false;
    } catch (error) {
      alert(error.message || "오류가 발생했습니다.");
      buyBtn.disabled = false;
    }
  };

  confirmBtn.onclick = async () => {
    if (pendingDecideResult !== null) {
      // 구매 후 확인 → 턴 마무리만 (알림은 구매 시 이미 표시됨)
      applyState(pendingDecideResult);
      pendingDecideResult = null;
      buyBtn.disabled = true;
      confirmBtn.disabled = true;
      diceEl.textContent = "-";

      if (!gameFinished) {
        rollBtn.disabled = false;
      }
    } else if (pendingTurnResult !== null) {
      // 이벤트/통행료 등 확인 후 턴 마무리
      applyState(pendingTurnResult);
      pendingTurnResult = null;
      buyBtn.disabled = true;
      confirmBtn.disabled = true;
      diceEl.textContent = "-";

      if (!gameFinished) {
        rollBtn.disabled = false;
      }
    } else {
      // 구매 안 함 → 서버에 skip 전달
      await sendDecide(false);
    }
  };

  accountBtn.onclick = async () => {
    if (!currentPlayerId) {
      return;
    }

    modal.classList.remove("hidden");
    await loadTransactions(currentPlayerId);
  };

  closeModal.onclick = () => {
    modal.classList.add("hidden");
  };
}

function showBuyDecision(price) {
  alert(`이 땅의 가격은 ${formatMoney(price)}입니다. 구매하려면 '토지 구매', 넘어가려면 '확인'을 누르세요.`);
  buyBtn.disabled = false;
  confirmBtn.disabled = false;
}

async function sendDecide(willBuy) {
  buyBtn.disabled = true;
  confirmBtn.disabled = true;
  pendingDecideResult = null;

  try {
    const response = await fetch("/api/decide", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ will_buy: willBuy }),
    });

    if (!response.ok) {
      const message = await response.text();
      throw new Error(message || "결정 처리 실패");
    }

    const result = await response.json();
    applyState(result);
    showTurnMessage(result);
    diceEl.textContent = "-";
  } catch (error) {
    alert(error.message || "오류가 발생했습니다.");
  } finally {
    if (!gameFinished) {
      rollBtn.disabled = false;
    }
  }
}

async function initGame() {
  try {
    const response = await fetch("/api/state");

    if (!response.ok) {
      const message = await response.text();
      throw new Error(message || "초기 상태 조회 실패");
    }

    const state = await response.json();
    applyState(state);
  } catch (error) {
    balanceEl.textContent = "잔액: 연결 실패";
    alert(error.message || "서버와 연결할 수 없습니다.");
  }
}

function applyState(state) {
  syncPlayers(state.players);
  currentPlayerId = state.current_player_id;
  gameFinished = state.game_finished;

  renderPlayers();
  updateTurnUI();
  updateBalance();

  if (gameFinished) {
    rollBtn.disabled = true;

    if (state.winner_id) {
      alert(`게임 종료! 승자는 Player ${state.winner_id} 입니다.`);
    }
  }
}

function syncPlayers(serverPlayers) {
  players.length = 0;

  serverPlayers.forEach((player) => {
    players.push({
      id: player.id,
      name: player.name,
      position: player.position,
      money: player.money,
      lap: player.lap,
      turnOrder: player.turn_order,
      isBankrupt: player.is_bankrupt,
    });

    updatePlayerLabel(player.id, player.name, player.money, player.is_bankrupt);
  });
}

function updatePlayerLabel(playerId, name, money, isBankrupt) {
  const label = document.querySelector(`.p${playerId}`);

  if (!label) {
    return;
  }

  const suffix = isBankrupt ? " (파산)" : "";
  label.textContent = `${name}${suffix}`;
}

function renderPlayers() {
  players.forEach((player) => {
    moveMarker(player.id, player.position);
  });
}

function moveMarker(playerId, position) {
  const marker = document.querySelector(`.player${playerId}`);
  const [row, col] = path[position];
  const cell = document.querySelector(`.tile[data-row="${row}"][data-col="${col}"]`);

  if (!marker || !cell) {
    return;
  }

  cell.appendChild(marker);
  arrangeMarkersInCell(row, col);
}

function arrangeMarkersInCell(row, col) {
  const cell = document.querySelector(`.tile[data-row="${row}"][data-col="${col}"]`);

  if (!cell) {
    return;
  }

  const markers = cell.querySelectorAll(".player-marker");
  const fixedPositions = {
    1: { x: -15, y: -15 }, // 좌상
    2: { x: 15, y: -15 },  // 우상
    3: { x: -15, y: 15 },  // 좌하
    4: { x: 15, y: 15 },   // 우하
  };

  markers.forEach((marker) => {
    // player1, player2 이런 클래스에서 숫자 추출
    const match = marker.className.match(/player(\d+)/);
    if (!match) return;

    const playerId = Number(match[1]);
    const pos = fixedPositions[playerId];

    if (!pos) return;

    marker.style.left = "50%";
    marker.style.top = "50%";
    marker.style.transform = `translate(-50%, -50%) translate(${pos.x}px, ${pos.y}px)`;
  });
}

function updateTurnUI() {
  const playerEls = document.querySelectorAll(".player");

  playerEls.forEach((element, index) => {
    const playerId = index + 1;

    if (playerId === currentPlayerId) {
      element.classList.add("active");
    } else {
      element.classList.remove("active");
    }
  });
}

function updateBalance() {
  const currentPlayer = players.find((player) => player.id === currentPlayerId) || players[0];

  if (!currentPlayer) {
    balanceEl.textContent = "잔액: -";
    return;
  }

  balanceEl.textContent = `잔액: ${formatMoney(currentPlayer.money)}`;
}

async function animateTurn(playerId, dice) {
  const player = players.find((entry) => entry.id === playerId);

  if (!player) {
    return;
  }

  isAnimating = true;

  for (let step = 0; step < dice; step += 1) {
    player.position = (player.position + 1) % path.length;
    moveMarker(player.id, player.position);
    await sleep(250);
  }

  isAnimating = false;
}

async function loadTransactions(playerId) {
  transactionBody.innerHTML = "";

  try {
    const response = await fetch(`/api/transactions/${playerId}`);

    if (!response.ok) {
      const message = await response.text();
      throw new Error(message || "거래 내역 조회 실패");
    }

    const transactions = await response.json();

    transactions.forEach((tx) => {
      const balanceBefore =
        tx.balance_before ?? tx.before_balance ?? tx.prev_balance ?? null;
      const balanceAfter =
        tx.balance_after ?? tx.after_balance ?? tx.next_balance ?? null;

      const row = document.createElement("tr");
      row.innerHTML = `
        <td>${tx.id}</td>
        <td>${formatTransactionType(tx.tx_type)}</td>
        <td>${formatMoney(tx.amount)}</td>
        <td>${formatMoney(balanceBefore)}</td>
        <td>${formatMoney(balanceAfter)}</td>
        <td>${tx.created_at}</td>
        <td>${tx.target}</td>
      `;
      transactionBody.appendChild(row);
    });
  } catch (error) {
    const row = document.createElement("tr");
    row.innerHTML = `<td colspan="7">${error.message || "거래 내역을 불러올 수 없습니다."}</td>`;
    transactionBody.appendChild(row);
  }
}

function showTurnMessage(result) {
  const messages = [];

  if (result.salary > 0) {
    messages.push(`월급 ${formatMoney(result.salary)} 지급`);
  }

  if (result.action_type === "purchase") {
    messages.push(`토지 구매 ${formatMoney(result.action_amount)}`);
  }

  if (result.action_type === "pay_toll") {
    messages.push(`통행료 ${formatMoney(result.action_amount)} 지불`);
  }

  if (result.action_type === "bankrupt") {
    messages.push(`파산 처리, ${formatMoney(result.action_amount)} 지급`);
  }

  if (messages.length > 0) {
    alert(messages.join("\n"));
  }
}

function formatMoney(amount) {
  const numericAmount = Number(amount);

  if (!Number.isFinite(numericAmount)) {
    return "-";
  }

  return `${numericAmount.toLocaleString()}만원`;
}

function formatTransactionType(txType) {
  if (txType === "deposit") {
    return "입금";
  }

  if (txType === "withdraw") {
    return "출금";
  }

  return txType;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}