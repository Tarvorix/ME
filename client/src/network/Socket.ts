import { encode, decode } from '@msgpack/msgpack';

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

export interface SocketConfig {
    url: string;
    reconnectInterval?: number;
    maxReconnectAttempts?: number;
    onMessage?: (data: unknown) => void;
    onStateChange?: (state: ConnectionState) => void;
}

/**
 * WebSocket wrapper with MessagePack encoding/decoding and auto-reconnect.
 */
export class GameSocket {
    private ws: WebSocket | null = null;
    private config: Required<SocketConfig>;
    private state: ConnectionState = 'disconnected';
    private reconnectAttempts = 0;
    private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    private latency = 0;
    private lastPingTime = 0;
    private pingInterval: ReturnType<typeof setInterval> | null = null;

    constructor(config: SocketConfig) {
        this.config = {
            url: config.url,
            reconnectInterval: config.reconnectInterval ?? 3000,
            maxReconnectAttempts: config.maxReconnectAttempts ?? 10,
            onMessage: config.onMessage ?? (() => {}),
            onStateChange: config.onStateChange ?? (() => {}),
        };
    }

    /** Connect to the WebSocket server. */
    connect(): void {
        if (this.ws) {
            this.ws.close();
        }

        this.setState('connecting');

        try {
            this.ws = new WebSocket(this.config.url);
            this.ws.binaryType = 'arraybuffer';

            this.ws.onopen = () => {
                this.setState('connected');
                this.reconnectAttempts = 0;
                this.startPing();
            };

            this.ws.onmessage = (event: MessageEvent) => {
                if (event.data instanceof ArrayBuffer) {
                    try {
                        const decoded = decode(new Uint8Array(event.data));
                        this.handleMessage(decoded);
                    } catch (err) {
                        console.error('Failed to decode message:', err);
                    }
                }
            };

            this.ws.onclose = () => {
                this.stopPing();
                this.setState('disconnected');
                this.tryReconnect();
            };

            this.ws.onerror = () => {
                this.setState('error');
            };
        } catch (err) {
            console.error('WebSocket connection failed:', err);
            this.setState('error');
            this.tryReconnect();
        }
    }

    /** Send a MessagePack-encoded message. */
    send(data: unknown): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            const encoded = encode(data);
            this.ws.send(encoded);
        }
    }

    /** Disconnect from the server. */
    disconnect(): void {
        this.stopPing();
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.setState('disconnected');
    }

    /** Get current connection state. */
    getState(): ConnectionState {
        return this.state;
    }

    /** Get current latency in milliseconds. */
    getLatency(): number {
        return this.latency;
    }

    /** Check if connected. */
    isConnected(): boolean {
        return this.state === 'connected';
    }

    private setState(state: ConnectionState): void {
        this.state = state;
        this.config.onStateChange(state);
    }

    private handleMessage(data: unknown): void {
        // Handle pong messages for latency calculation
        if (data && typeof data === 'object' && 'Pong' in (data as Record<string, unknown>)) {
            this.latency = performance.now() - this.lastPingTime;
            return;
        }

        this.config.onMessage(data);
    }

    private tryReconnect(): void {
        if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
            console.warn('Max reconnect attempts reached');
            return;
        }

        this.reconnectAttempts++;
        this.reconnectTimer = setTimeout(() => {
            console.log(`Reconnecting (attempt ${this.reconnectAttempts})...`);
            this.connect();
        }, this.config.reconnectInterval);
    }

    private startPing(): void {
        this.pingInterval = setInterval(() => {
            if (this.isConnected()) {
                this.lastPingTime = performance.now();
                this.send({ Ping: null });
            }
        }, 5000);
    }

    private stopPing(): void {
        if (this.pingInterval) {
            clearInterval(this.pingInterval);
            this.pingInterval = null;
        }
    }
}
