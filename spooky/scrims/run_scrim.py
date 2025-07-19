import subprocess
import os
import sys
import toml
import time
import select
from datetime import datetime

# Add the spooky directory to the python path to allow importing ratings
SPOOKY_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.append(SPOOKY_DIR)

from scrims.ratings import get_rating, update_ratings

class EnginePanicError(Exception):
    """Custom exception for when the engine process panics."""
    pass

class UmiProcess:
    """A wrapper for a subprocess running a UMI-compatible chess engine."""
    def __init__(self, path):
        self.path = os.path.abspath(path)
        self.name = os.path.basename(path)
        self.proc = subprocess.Popen(
            [self.path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            shell=True,
            bufsize=1,
        )
        self._send_command("umi")
        self._wait_for_response("umiok")

    def _send_command(self, command):
        print(f"> {self.name}: {command}")
        self.proc.stdin.write(command + '\n')
        self.proc.stdin.flush()

    def _wait_for_response(self, expected_prefix):
        lines = []
        while True:
            line = self.proc.stdout.readline().strip()
            if line:
                lines.append(line)
                print(f"< {self.name}: {line}")
                if line.startswith(expected_prefix):
                    return lines

    def set_position(self, fen):
        if fen == "startpos":
            self._send_command("position startpos")
        else:
            self._send_command(f"position fen {fen}")

    def play(self, time_control):
        self._send_command(f"play {time_control}")
        
        info_lines = self._wait_for_response("turn")
        turn_line = info_lines.pop()
        turn_lines = [turn_line]
        turn_lines.extend(self._wait_for_response("endturn"))

        winner_side = None
        if 'winner' in turn_lines[-1]:
            winner_side = turn_lines[-1].split(' ')[-1]

        return turn_lines, info_lines, winner_side

    def quit(self):
        self._send_command("quit")
        self.proc.wait()

def run_game(yellow_ai, blue_ai, time_control, start_fen, match_log_path):
    """Run a single game between two AIs."""
    yellow_ai.set_position(start_fen)
    blue_ai.set_position(start_fen)

    current_player = yellow_ai
    other_player = blue_ai
    turn_num = 1

    winner = 'draw' # Default to draw if game exits unexpectedly

    while True:
        try:
            turn_lines, info_lines, declared_winner = current_player.play(time_control)
        except EnginePanicError:
            winner = other_player.name
            break

        # Check for game over
        if declared_winner:
            if declared_winner.lower() == 'yellow':
                winner = yellow_ai.name
            else:
                winner = blue_ai.name
            break
        if len(turn_lines) <= 2: # An empty turn block means no legal moves
            winner = other_player.name
            break

        # Send the entire turn block to the other AI
        for line in turn_lines:
            other_player._send_command(line)

        # Log the turn with metadata
        with open(match_log_path, 'a') as f:
            info_metadata = " ".join([f"[{info}]" for info in info_lines])
            log_entry = turn_lines[0]
            if info_metadata:
                log_entry += ' ' + info_metadata
            log_entry += '\n' + '\n'.join(turn_lines[1:])
            f.write(log_entry + '\n\n')

        # Swap players
        current_player, other_player = other_player, current_player
        turn_num += 1

        # Simple draw condition
        if turn_num > 200:
            winner = "draw"
            break

    with open(match_log_path, 'a') as f:
        f.write(f"Winner: {winner}\n")

    return winner

def main(config_path):
    config = toml.load(config_path)

    # Setup paths and directories
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    match_name = config['match']['name']
    match_dir = os.path.join(SPOOKY_DIR, 'matches', f"{match_name}-{timestamp}")
    results_file = os.path.join(SPOOKY_DIR, 'scrims', 'results', f"{match_name}-{timestamp}.txt")
    os.makedirs(match_dir, exist_ok=True)

    # Initialize AIs
    yellow_path = os.path.join(SPOOKY_DIR, config['ai_yellow']['path'])
    blue_path = os.path.join(SPOOKY_DIR, config['ai_blue']['path'])
    yellow_ai = UmiProcess(yellow_path)
    blue_ai = UmiProcess(blue_path)

    yellow_name = os.path.basename(yellow_ai.path)
    blue_name = os.path.basename(blue_ai.path)

    print(f"Starting scrimmage: {yellow_name} vs {blue_name}")
    print(f"Yellow ELO: {get_rating(yellow_name, SPOOKY_DIR)}")
    print(f"Blue ELO: {get_rating(blue_name, SPOOKY_DIR)}")

    scores = {yellow_name: 0, blue_name: 0, 'draw': 0}

    for i in range(config['match']['num_games']):
        print(f"\n--- Game {i+1} of {config['match']['num_games']} ---")
        match_log_path = os.path.join(match_dir, f"{yellow_name}-vs-{blue_name}-{i+1}.minions")

        # Alternate starting player
        if i % 2 == 0:
            winner = run_game(yellow_ai, blue_ai, config['match']['time_control'], config['match']['start_fen'], match_log_path)
        else:
            winner = run_game(blue_ai, yellow_ai, config['match']['time_control'], config['match']['start_fen'], match_log_path)

        scores[winner] += 1
        print(f"Game {i+1} winner: {winner}")

        # Update ratings if not a draw
        if winner != 'draw':
            loser = blue_name if winner == yellow_name else yellow_name
            # Don't update ratings for the dev build
            if 'target' not in yellow_ai.path and 'target' not in blue_ai.path:
                 new_winner_rating, new_loser_rating = update_ratings(winner, loser, SPOOKY_DIR)
                 print(f"New ratings: {winner}: {new_winner_rating}, {loser}: {new_loser_rating}")

    # Final results
    summary = f"Final Score:\n{yellow_name}: {scores[yellow_name]}\n{blue_name}: {scores[blue_name]}\nDraws: {scores['draw']}"
    print("\n" + summary)
    with open(results_file, 'w') as f:
        f.write(summary)

    yellow_ai.quit()
    blue_ai.quit()

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: python {sys.argv[0]} <path_to_config.toml>")
        sys.exit(1)
    main(sys.argv[1])
