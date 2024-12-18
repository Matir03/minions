Search:

- MCTS for overall search  
  - static evaluator with confidence interval for pruning "bad" explores  
- expand ply by subply "stages":  
  - generaling stage  
  - attack stage  
  - blotto stage  
  - spawn stage  
- generaling and attack stage independent, blotto stage dependent on both, spawn stage dependent on all  
- each stage tracks backpropagated feedback  
- stages lazily generate refined candidate moves

General:

- decide which unit(s) to tech to  
- just generate all possible choices at this point  
- use NN for initial choice   
- later, deal with triangles  
  - use refinement based approach at this point

Attack:

- for each friendly piece, compute all pieces they can hit (possibly from hexes blocked by enemy pieces), to generate all attacker-defender pairs (and all possible attack hexes for each such pair)  
- generate constraint graph with following attributes:  
  - variables  
    - for each defending piece y  
      - bool r\_y signifying whether y is removed  
      - int tr\_y denoting the time y is removed  
      - bool k\_y signifying whether y is killed  
      - bool u\_y signifying whether y is unsummoned  
    - for each attacking piece x   
      - bool p\_x denoting whether x is passive (i.e. does not engage in combat)  
      - int ta\_x denoting the time x engages in combat  
      - bool m\_xs for each hex s in its list of attack hexes denoting whether x moved to s  
    - for each attacker/defender pair x, y  
      - bool a\_xy denoting whether x attacked y  
      - int d\_xy denoting the effective damage dealt by x to y  
  - constraints  
    - on hexes: for every relevant hex s  
      - atMostOne(m\_xs)   
      - m\_xs \=\>  
        - f\_x({ta\_x \> tr\_y: y relevant in blocking m\_xs}) for boolean formula f\_x depending on the movement    
        - ta\_x \> ta\_z if z is an attacking piece starting on s  
    - on attacking pieces: for every attack-relevant piece x,   
      - movement: exactlyOne(p\_x, m\_xs)  
      - range: a\_xy \=\> exactlyOne(m\_xs, s within range of y)  
      - timing: a\_xy \=\> ta\_x \<= tr\_y  
      - num attacks: num(a\_xy) \<= maxAttacks(x)   
      - damage  
        - if x has flurry: sum(d\_xy) \<= maxDamage(x)  
        - if x has \* attack and y is persistent: d\_xy \= a\_xy  
        - if x has deathtouch attack and y is necromancer: d\_xy \= 0  
        - otherwise: d\_xy \= a\_xy \* attack(x)  
          - where attack(x) \= inf if x has unsummon or deathtouch attack   
    - on defending pieces:  
      - remove/kill/unsummon: exactlyOne(\~r\_y, k\_y, u\_y)  
      - unsummon: u\_y \= exactlyOne(a\_xy where x has \* attack)  
      - removal: let d\_y \= sum(d\_xy)  
        - r\_y \= (d\_y \>= defense(y))	  
        - (optional) no overkill: exists x, d\_y \- d\_xy \< defense(y)

      

- use heuristics (e.g. a neural network) to identify a set of k (\~3-6) "prophecies", where each prophecy is a map from the set of attacking and defending pieces to probabilities that they are passive/removed respectively  
- deterministically generate a candidate move from a prophecy as follows:  
  - sort all pieces with probability above a threshold (e.g. ½) in decreasing order of predicted probability   
  - initialize a solver with the above constraints, and iterate through the sorted list of pieces, adding the constraint p\_x or r\_y as appropriate. if adding the constraint results in unsat, backtrack and skip the constraint.   
  - generate a model for the final problem (known to be sat), and translate the model into a move  
  - some friendly pieces may need to be "evacuated" (moved out of the way) to make the move legal. verify that such an evacuation is possible.  
- every time a new candidate move is generated add it to a list of constraints disallowing already generated moves   
- use NNUE to evaluate post-attack position; small changes computed efficiently  
  - post-attack position evaluated in the context of the original position, i.e. evaluation means "expected score of global position given this board plays this move and other boards play optimally conditioned on this move"  
- use NNUE and MCTS values and confidence to choose a candidate move on successive explorations, possibly exploring a coarse or fine refinement of the candidate move  
- to generate a fine refinement, take the current solver state for the candidate move and add the list of constraints corresponding to already generated moves. if this is sat, take the new move and make it a candidate move. otherwise, report failure to the MCTS algorithm, which will set fine refinement probability on the corresponding candidate move family  
- to generate a coarse refinement, add random noise to the prophecy to generate a new prophecy, and use the new prophecy to generate a candidate (still using the disallowed moves constraint)    
- maybe do something with the lookahead from MCTS backpropagation?

Spawn:

- compute candidate spawn location \+ unit combos for spawn stage  
- tabulate combo: heuristic goodness, price values  
- use heuristics/NN to generate map for each board from dollar amount k \-\> expected score of global position given k dollars blotto'd to given board  
- update score and confidence for each board individually when backpropping from MCTS  
- (max, \+) convolve keeping track of argmaxes to choose new blotto  
  - add random noise based on confidence for stochastic blotto  
- choose spawning combo for given blotto amount stochastically from spawn nodes  
- move remaining movable pieces to optimal locations based on some heuristic

Neural Networks:

- Death Prophet: predict attacks  
- Dark Seer: predict piece placement/spawning  
- Oracle: evaluate position

Core Representations:

- Game: GameConfig \+ GameState  
- GameConfig:   
  - int numBoards  
  - MapLabel\[numBoards\] maps  
    - MapLabel: enum { BlackenedShores, MidnightLake, … }  
    - into global Vector\<Map\> as map from MapLabel to Map  
  - Techline techline  
- GameState:  
  - Side side_to_move;
  - Board\[numBoards\] boards  
  - SideArray\<TechStatus\[numTechs\]\> techStatus  
    - TechStatus \= Locked | Unlocked | Acquired  
  - SideArray\<int\> money  
- Map: HexArray\<TileType\> hexes  
- TileType: Land | Water | Graveyard | …   
- Techline:  
  - int numTechs  
  - int\[numTechs\] Tech  
- Tech: UnitTech(UnitLabel) | Copycat | Thaumaturgy | Metamagic  
  - UnitLabel: enum { Zombie, Initiate, … }  
  - into global Vector\<Unit\> units as map from UnitLabel to Unit  
- Unit:  
  - Attack attack  
    - Attack \= Damage(int) | Unsummon | Deathtouch  
  - int numAttack  
  - int defense  
  - int speed  
  - int range  
  - int cost  
  - int rebate  
  - bool necromancer  
  - bool lumbering  
  - bool flying  
  - bool persistent  
  - bool spawn  
- Board:  
  - SideArray\<Vector\<Piece\>\> pieces  
  - SideArray\<Vector\<UnitLabel\>\> reinforcements  
  - SideArray\<Vector\<Spell\>\> spells  
- Piece:  
  - Loc loc  
  - UnitLabel unit  
  - Modifiers modifiers  
    - Modifiers \= { bool shielded; bool frozen; bool shackled; … }  
- Spell \= Shield | Reposition | …  
- GameTurn:  
  - int num_spells_bought
  - int\[num_boards\] board_spells \\ board -> index of drawn spell
  - Vector<int> tech_spells \\ techline indices of tech spells 
  - Vector<BoardAction> board_actions[num_boards]
- BoardAction:  
  - Move   
    - Loc from\_sq  
    - Loc to\_sq  
  - Attack  
    - Loc from\_sq  
    - Loc to\_sq  
  - Spawn  
    - Loc spawn\_sq  
    - UnitLabel unit  
  - Cast  
    - Spell spell  
    - CastParams params  
  - Discard  
    - Spell spell  
  - EndPhase
