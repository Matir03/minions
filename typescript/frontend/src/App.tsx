import { useState } from 'react'
import './App.css'
import { useEngine } from './hooks/useEngine'

function App() {
  const { sendCommand } = useEngine();

  const testEngine = () => {
    sendCommand('isready');
  };

  return (
    <div className="app">
      <h1>Minions Board Game</h1>
      <button onClick={testEngine}>Test Engine Connection</button>
      <div className="game-board">
        {/* Game board will go here */}
      </div>
    </div>
  )
}

export default App
