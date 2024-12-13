import { spawn, type ChildProcessWithoutNullStreams } from 'child_process';
import { EventEmitter } from 'events';

export class UCIEngine extends EventEmitter {
  private process: ChildProcessWithoutNullStreams | null = null;
  private initialized = false;

  constructor(private enginePath: string) {
    super();
  }

  async start() {
    this.process = spawn(this.enginePath);
    
    this.process.stdout.on('data', (data) => {
      const lines = data.toString().trim().split('\n');
      for (const line of lines) {
        this.emit('engineOutput', line.trim());
      }
    });

    this.process.stderr.on('data', (data) => {
      console.error(`Engine error: ${data}`);
    });

    this.process.on('close', (code) => {
      console.log(`Engine process exited with code ${code}`);
      this.initialized = false;
      this.process = null;
    });

    // Initialize UCI mode
    await this.sendCommand('uci');
    await this.sendCommand('isready');
    this.initialized = true;
  }

  async sendCommand(command: string): Promise<void> {
    if (!this.process) {
      throw new Error('Engine not started');
    }

    return new Promise((resolve) => {
      this.process!.stdin.write(command + '\n');
      resolve();
    });
  }

  async quit() {
    if (this.process) {
      await this.sendCommand('quit');
      this.process = null;
      this.initialized = false;
    }
  }

  isInitialized(): boolean {
    return this.initialized;
  }
}
