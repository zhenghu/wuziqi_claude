// ---------------- AI (与 Rust 版战术搜索一致) ----------------
// 由 wuziqi.html 提供棋盘常量、方向表以及 opponent / inBoard。
const ROOT_CANDIDATE_LIMIT=12, REPLY_CANDIDATE_LIMIT=10, MATE_SCORE=1000000000000;

function lineStat(board,x,y,dx,dy,p){
  let count=1, open=0;
  for(const dir of [1,-1]){
    let cx=x+dx*dir, cy=y+dy*dir;
    while(inBoard(cx,cy)&&board[cy][cx]===p){ count++; cx+=dx*dir; cy+=dy*dir; }
    if(inBoard(cx,cy)&&board[cy][cx]===EMPTY) open++;
  }
  return [count,open];
}
function patternScore(count,open){
  if(count>=5) return 10000000;
  if(count===4&&open===2) return 1000000;
  if(count===4&&open===1) return 120000;
  if(count===3&&open===2) return 60000;
  if(count===3&&open===1) return 2000;
  if(count===2&&open===2) return 800;
  if(count===2&&open===1) return 150;
  if(count===1&&open===2) return 40;
  if(count===1&&open===1) return 10;
  return 0;
}
function pointScore(board,x,y,p){
  let total=0;
  for(const [dx,dy] of DIRECTIONS){
    const [c,o]=lineStat(board,x,y,dx,dy,p);
    total+=patternScore(c,o);
  }
  return total;
}
function nearStone(board,x,y){
  for(let dy=-2;dy<=2;dy++) for(let dx=-2;dx<=2;dx++){
    const cx=x+dx, cy=y+dy;
    if(inBoard(cx,cy)&&board[cy][cx]!==EMPTY) return true;
  }
  return false;
}

function candidateMoves(board){
  const moves=[];
  for(let y=0;y<BOARD;y++) for(let x=0;x<BOARD;x++){
    if(board[y][x]===EMPTY&&nearStone(board,x,y)) moves.push([x,y]);
  }
  if(moves.length===0&&board[CENTER][CENTER]===EMPTY) moves.push([CENTER,CENTER]);
  return moves;
}

function isWinningMove(board,x,y,p){
  if(board[y][x]!==EMPTY) return false;
  for(const [dx,dy] of DIRECTIONS){
    if(lineStat(board,x,y,dx,dy,p)[0]>=5) return true;
  }
  return false;
}

function immediateWinningMoves(board,p){
  return candidateMoves(board).filter(([x,y])=>isWinningMove(board,x,y,p));
}

function countImmediateWins(board,p,stopAfter){
  let count=0;
  for(const [x,y] of candidateMoves(board)){
    if(isWinningMove(board,x,y,p)){
      count++;
      if(count===stopAfter) break;
    }
  }
  return count;
}

function doubleThreatMoves(board,p){
  const threats=[];
  for(const [x,y] of candidateMoves(board)){
    if(isWinningMove(board,x,y,p)) continue;
    board[y][x]=p;
    const isDouble=countImmediateWins(board,p,2)>=2;
    board[y][x]=EMPTY;
    if(isDouble) threats.push([x,y]);
  }
  return threats;
}

function moveOrderScore(board,x,y,p){
  const attack=pointScore(board,x,y,p);
  const defend=pointScore(board,x,y,opponent(p));
  const centerBias=BOARD-1-(Math.abs(x-CENTER)+Math.abs(y-CENTER));
  return attack*10+defend*9+centerBias;
}

function rankedMoves(board,p,limit,required){
  const requiredKeys=new Set(required.map(([x,y])=>y*BOARD+x));
  const scored=candidateMoves(board).map(([x,y])=>({x,y,score:moveOrderScore(board,x,y,p)}));
  scored.sort((a,b)=>b.score-a.score||a.y-b.y||a.x-b.x);

  const selected=[], selectedKeys=new Set();
  for(const move of scored){
    const key=move.y*BOARD+move.x;
    if(requiredKeys.has(key)){
      selected.push([move.x,move.y]);
      selectedKeys.add(key);
    }
  }
  let regular=0;
  for(const move of scored){
    const key=move.y*BOARD+move.x;
    if(selectedKeys.has(key)) continue;
    if(regular===limit) break;
    selected.push([move.x,move.y]);
    regular++;
  }
  return selected;
}

function threatValue(board,p){
  let first=0, second=0;
  for(const [x,y] of candidateMoves(board)){
    const score=pointScore(board,x,y,p);
    if(score>first){ second=first; first=score; }
    else if(score>second){ second=score; }
  }
  return first*4+second;
}

function evaluatePosition(board,ai){
  return threatValue(board,ai)*10-threatValue(board,opponent(ai))*9;
}

function leafScore(board,ai){
  if(immediateWinningMoves(board,ai).length>0) return MATE_SCORE;

  const human=opponent(ai);
  const humanWins=immediateWinningMoves(board,human);
  if(humanWins.length>=2) return -MATE_SCORE;
  if(humanWins.length===1){
    const [x,y]=humanWins[0];
    board[y][x]=ai;
    const score=evaluatePosition(board,ai);
    board[y][x]=EMPTY;
    return score;
  }
  return evaluatePosition(board,ai);
}

function replyScore(board,ai,alpha,humanForks){
  const human=opponent(ai);
  if(immediateWinningMoves(board,human).length>0) return -MATE_SCORE;

  const aiWins=immediateWinningMoves(board,ai);
  if(aiWins.length>=2) return MATE_SCORE;

  let replies;
  if(aiWins.length===1){
    replies=aiWins;
  } else {
    // AI 新增的棋子只会消除既有玩家双杀，不会创造新的玩家双杀
    replies=rankedMoves(board,human,REPLY_CANDIDATE_LIMIT,humanForks);
  }
  if(replies.length===0) return evaluatePosition(board,ai);

  let worst=MATE_SCORE*2;
  for(const [x,y] of replies){
    const winsNow=isWinningMove(board,x,y,human);
    board[y][x]=human;
    const score=winsNow?-MATE_SCORE:leafScore(board,ai);
    board[y][x]=EMPTY;
    worst=Math.min(worst,score);
    if(worst<=alpha) break;
  }
  return worst;
}

function aiMove(board,ai,moveCount){
  if(moveCount===0) return [CENTER,CENTER];
  const searchBoard=board.map(row=>row.slice());
  const human=opponent(ai);

  const aiWins=immediateWinningMoves(searchBoard,ai);
  if(aiWins.length>0) return rankedMoves(searchBoard,ai,0,aiWins)[0];

  const humanWins=immediateWinningMoves(searchBoard,human);
  if(humanWins.length>0) return rankedMoves(searchBoard,ai,0,humanWins)[0];

  const aiForks=doubleThreatMoves(searchBoard,ai);
  if(aiForks.length>0) return rankedMoves(searchBoard,ai,0,aiForks)[0];

  const humanForks=doubleThreatMoves(searchBoard,human);
  if(humanForks.length===1) return humanForks[0];
  const roots=rankedMoves(searchBoard,ai,ROOT_CANDIDATE_LIMIT,humanForks);
  if(roots.length===0) return [CENTER,CENTER];

  let best=roots[0], bestScore=-MATE_SCORE*2;
  for(const [x,y] of roots){
    searchBoard[y][x]=ai;
    const score=replyScore(searchBoard,ai,bestScore,humanForks);
    searchBoard[y][x]=EMPTY;
    if(score>bestScore){ bestScore=score; best=[x,y]; }
  }
  return best;
}
