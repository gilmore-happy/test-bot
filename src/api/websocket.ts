import { WebSocketMessage } from '../types';

type MessageHandler<T> = (data: T) => void;
type ErrorHandler = (error: Event) => void;
type ConnectionHandler = () => void;

class WebSocketService {
  private socket: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectTimeout = 1000; // Start with 1 second
  private messageHandlers: Map<string, MessageHandler<any>[]> = new Map();
  private onConnectHandlers: ConnectionHandler[] = [];
  private onDisconnectHandlers: ConnectionHandler[] = [];
  private onErrorHandlers: ErrorHandler[] = [];
  private url: string;

  constructor(url: string) {
    this.url = url;
  }

  public connect(): void {
    if (this.socket?.readyState === WebSocket.OPEN) {
      return;
    }

    this.socket = new WebSocket(this.url);

    this.socket.onopen = () => {
      console.log('WebSocket connected');
      this.reconnectAttempts = 0;
      this.onConnectHandlers.forEach(handler => handler());
    };

    this.socket.onmessage = (event: MessageEvent) => {
      try {
        const message = JSON.parse(event.data) as WebSocketMessage<any>;
        const handlers = this.messageHandlers.get(message.type) || [];
        handlers.forEach(handler => handler(message.data));
      } catch (error) {
        console.error('Error parsing WebSocket message:', error);
      }
    };

    this.socket.onclose = () => {
      console.log('WebSocket disconnected');
      this.onDisconnectHandlers.forEach(handler => handler());
      this.attemptReconnect();
    };

    this.socket.onerror = (error: Event) => {
      console.error('WebSocket error:', error);
      this.onErrorHandlers.forEach(handler => handler(error));
    };
  }

  public disconnect(): void {
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
  }

  public on<T>(messageType: string, handler: MessageHandler<T>): void {
    const handlers = this.messageHandlers.get(messageType) || [];
    handlers.push(handler);
    this.messageHandlers.set(messageType, handlers);
  }

  public off<T>(messageType: string, handler: MessageHandler<T>): void {
    const handlers = this.messageHandlers.get(messageType) || [];
    const index = handlers.indexOf(handler);
    if (index !== -1) {
      handlers.splice(index, 1);
      this.messageHandlers.set(messageType, handlers);
    }
  }

  public onConnect(handler: ConnectionHandler): void {
    this.onConnectHandlers.push(handler);
  }

  public onDisconnect(handler: ConnectionHandler): void {
    this.onDisconnectHandlers.push(handler);
  }

  public onError(handler: ErrorHandler): void {
    this.onErrorHandlers.push(handler);
  }

  public send(data: any): void {
    if (this.socket?.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify(data));
    } else {
      console.error('WebSocket is not connected');
    }
  }

  // Add a method to check connection status
  public isConnected(): boolean {
    return this.socket?.readyState === WebSocket.OPEN;
  }

  // Add a method to get connection state
  public getConnectionState(): number | null {
    return this.socket?.readyState ?? null;
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnect attempts reached');
      return;
    }

    this.reconnectAttempts++;
    const timeout = this.reconnectTimeout * Math.pow(2, this.reconnectAttempts - 1);
    
    console.log(`Attempting to reconnect in ${timeout}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
    
    setTimeout(() => {
      this.connect();
    }, timeout);
  }
}

// Create a singleton instance
export const webSocketService = new WebSocketService(
  process.env.REACT_APP_WS_URL || 'ws://localhost:8080/ws'
);

// Export a hook to use the WebSocket service
export const useWebSocket = () => {
  return webSocketService;
};