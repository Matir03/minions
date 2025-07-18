# AI Strategy

This document outlines the specific heuristics and decision-making logic for the AI.

## Blotto

*   General gets as many spells as requested.
*   Board gets money split evenly.

## Generaling

*   Prioritize getting a counter to the highest uncountered enemy unit.
*   If all enemy units are countered, prioritize getting n+3 of the current highest unit if it is available; if the opponent has it, get n+5 instead. If neither are possible (i.e., out of the tech line), tech to the highest unit available.
*   If the target can be bought today, do so; otherwise, march.

## Attacking

*   Kill > Unsummon > Keep for all units.
*   First, prioritize unit value: Necro > C-R > unit number.
*   Second, prioritize closeness to the graveyard.

## Positioning

*   Compute "sidedness" for each graveyard by the difference in the (exponentially weighted distance * dollar value) average of each side's pieces to the graveyard.
*   **Occupying**: For graveyards leaning towards us, iterate through graveyards in increasing order of sidedness (i.e., most heavily us-favored to most neutral) and assign an "occupier" for each graveyard.
    *   For the occupier, choose the cheapest unit that can get there in the fewest turns.
*   Have a formula for converting "sidedness" to "competitiveness points."
*   In increasing order of competitiveness:
    *   Choose a spawner: the highest-numbered unit that can get there in the fewest turns.
*   All chosen units assign priority by how well the move gets them to the target graveyard (1 - n * eps for n-maximally close -> 0 + n * eps for n-maximally far).
*   Go through remaining units. For each move of that unit, value that move at delta(total competitiveness score).

## Spawning

*   For each enemy unit type that we can buy counters for, compute the counters:unit money on board ratio. Buy the highest counter for the unit with the lowest such ratio.
*   Choose a location that maximizes delta(competitiveness).

## Eval

*   Compute total dollar diff:
    *   Dollars on boards
    *   Dollars on tech line
    *   Board points at some constant dollar value (say 30).
*   Convert dollar diff to eval by, e.g., a scaled sigmoid.
