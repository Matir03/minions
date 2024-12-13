import express from 'express';
import { createServer } from 'http';
import { WebSocketServer } from 'ws';
import cors from 'cors';
import { UCIEngine } from './engine.js';
import { join } from 'path';

const app = express();
const server = createServer(app);
const wss = new WebSocketServer({ server });

app.use(cors());
app.use(express.json());

const engine = new UCIEngine(join(process.cwd(), '../../rust/target/debug/minions-uci'));

// WebSocket connection handling
wss.on('connection', (ws) => {
  console.log('Client connected');

  // Forward engine output to the client
  engine.on('engineOutput', (output: string) => {
    ws.send(JSON.stringify({ type: 'engineOutput', data: output }));
  });

  ws.on('message', async (message) => {
    try {
      const { type, command } = JSON.parse(message.toString());
      
      if (type === 'command') {
        await engine.sendCommand(command);
      }
    } catch (error) {
      console.error('Error processing message:', error);
    }
  });

  ws.on('close', () => {
    console.log('Client disconnected');
  });
});

// Start the engine when the server starts
server.listen(3000, async () => {
  console.log('Server running on port 3000');
  try {
    await engine.start();
    console.log('Engine started successfully');
  } catch (error) {
    console.error('Failed to start engine:', error);
  }
});
