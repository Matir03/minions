# Search System

The Spooky engine uses Monte Carlo Tree Search (MCTS) as its primary search algorithm, enhanced with several sophisticated components for improved performance.

## MCTS Implementation

### Core Components
- Tree node structure tracking move sequences and outcomes
- UCT-style selection with confidence interval modifications
- Backpropagation of evaluation scores and confidence metrics
- Multi-stage expansion process

### Static Evaluator
- Neural network-based position evaluation
- Confidence interval generation for move quality
- Pruning of low-confidence or poor-quality moves
- Integration with NNUE for efficient updates

## Stage-Based Expansion

The search process expands nodes through multiple stages:

1. **Ply Expansion**
   - Each ply is expanded through subply stages
   - Stages are processed in fixed order: general → attack → blotto → spawn
   - Each stage maintains independent feedback metrics

2. **Stage Dependencies**
   - General and attack stages operate independently
   - Blotto stage depends on general and attack outcomes
   - Spawn stage depends on all previous stage outcomes

3. **Feedback Integration**
   - Each stage tracks and backpropagates specific feedback
   - Feedback influences future expansions and refinements
   - Cross-stage feedback coordination for optimal decisions

## Move Refinement

### Candidate Generation
- Stages lazily generate refined candidate moves
- Initial coarse moves based on neural network guidance
- Progressive refinement based on search feedback
- Constraint satisfaction for move validity

### NNUE Integration
- Efficient incremental position evaluation
- Context-aware scoring considering global position
- Fast updates for small board changes
- Confidence metrics for evaluation reliability

## Performance Optimizations

### Pruning Strategies
- Confidence interval-based exploration pruning
- Early termination of unpromising branches
- Efficient memory management for large trees

### Parallel Processing
- Independent stage processing where possible
- Efficient state sharing between parallel processes
- Lock-free data structures for high throughput
