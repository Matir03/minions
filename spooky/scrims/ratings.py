import json
import os

ELO_FILE = 'ratings/elo_ratings.json'
DEFAULT_ELO = 1200
K_FACTOR = 32

def get_ratings(spooky_dir):
    """Load ELO ratings from the JSON file."""
    ratings_path = os.path.join(spooky_dir, ELO_FILE)
    if not os.path.exists(ratings_path):
        return {}
    with open(ratings_path, 'r') as f:
        return json.load(f)

def save_ratings(ratings, spooky_dir):
    """Save ELO ratings to the JSON file."""
    ratings_path = os.path.join(spooky_dir, ELO_FILE)
    os.makedirs(os.path.dirname(ratings_path), exist_ok=True)
    with open(ratings_path, 'w') as f:
        json.dump(ratings, f, indent=4)

def update_ratings(winner_name, loser_name, spooky_dir):
    """Update ELO ratings based on a match result."""
    ratings = get_ratings(spooky_dir)

    winner_rating = ratings.get(winner_name, DEFAULT_ELO)
    loser_rating = ratings.get(loser_name, DEFAULT_ELO)

    # Calculate expected scores
    expected_winner = 1 / (1 + 10 ** ((loser_rating - winner_rating) / 400))
    expected_loser = 1 / (1 + 10 ** ((winner_rating - loser_rating) / 400))

    # Update ratings
    new_winner_rating = winner_rating + K_FACTOR * (1 - expected_winner)
    new_loser_rating = loser_rating + K_FACTOR * (0 - expected_loser)

    ratings[winner_name] = round(new_winner_rating)
    ratings[loser_name] = round(new_loser_rating)

    save_ratings(ratings, spooky_dir)

    return ratings[winner_name], ratings[loser_name]

def get_rating(ai_name, spooky_dir):
    """Get the rating for a single AI."""
    ratings = get_ratings(spooky_dir)
    return ratings.get(ai_name, DEFAULT_ELO)
