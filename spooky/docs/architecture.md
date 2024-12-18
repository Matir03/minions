# Architecture Overview

The Spooky engine is built around a sophisticated Monte Carlo Tree Search (MCTS) implementation that breaks down move generation into distinct stages. This document provides a high-level overview of the system architecture.

## Core Components

### Search System
- MCTS-based search with static evaluation
- Confidence interval-based pruning for exploration optimization
- Neural network evaluation (NNUE) for position assessment
- Backpropagation of feedback through all stages

### Move Generation Pipeline
The move generation process is divided into four sequential stages:

1. **General Stage**
   - Tech decisions
   - Initial candidate generation using neural networks

2. **Attack Stage**
   - Combat resolution through constraint satisfaction
   - Attacker-defender pair identification
   - Movement and timing coordination
   - Neural network-based "prophecy" generation

3. **Blotto Stage**
   - Resource allocation decisions
   - Dependent on general and attack stage outcomes
   - Optimization of resource distribution

4. **Spawn Stage**
   - Unit placement decisions
   - Dependent on all previous stages
   - Position optimization

## Key Features

### Lazy Refinement
- Each stage implements lazy generation of refined candidate moves
- Coarse to fine refinement based on MCTS feedback
- Efficient pruning of unpromising branches

### Neural Network Integration
- Static evaluation with confidence intervals
- Initial move candidate generation
- Position evaluation using NNUE
- Efficient incremental updates for small position changes

### Constraint Satisfaction
- Robust constraint graph system for combat resolution
- Efficient SAT solver integration
- Handling of complex game rules and interactions

## Implementation Notes

- Each stage maintains its own state and feedback mechanisms
- Stages can operate independently where possible
- Careful coordination of dependencies between stages
- Efficient caching and incremental updates throughout the pipeline
