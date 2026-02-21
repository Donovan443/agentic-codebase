/**
 * A simple TypeScript module for testing the parser.
 */

import { EventEmitter } from 'events';
import * as fs from 'fs';

/** Interface for a logger. */
export interface Logger {
    log(message: string): void;
    error(message: string): void;
}

/** Type alias for configuration. */
export type Config = {
    name: string;
    debug: boolean;
    options?: Record<string, unknown>;
};

/** Base class for services. */
export class BaseService {
    protected name: string;

    constructor(name: string) {
        this.name = name;
    }

    /** Get the service name. */
    getName(): string {
        return this.name;
    }

    /** Start the service. */
    async start(): Promise<void> {
        console.log(`Starting ${this.name}`);
    }
}

/** A service that extends BaseService. */
export class DataService extends BaseService {
    private data: Map<string, unknown>;

    constructor(name: string) {
        super(name);
        this.data = new Map();
    }

    /** Fetch data from a source. */
    async fetchData(url: string): Promise<unknown> {
        const response = await fetch(url);
        return response.json();
    }

    /** Process data items. */
    processItems(items: string[]): number {
        let count = 0;
        for (const item of items) {
            if (item.startsWith('a')) {
                count++;
            }
        }
        return count;
    }
}

/** A standalone function. */
export function createLogger(name: string): Logger {
    return {
        log: (message: string) => console.log(`[${name}] ${message}`),
        error: (message: string) => console.error(`[${name}] ${message}`),
    };
}

/** Arrow function assigned to const. */
export const processData = (data: unknown[]): unknown[] => {
    return data.filter(item => item !== null);
};

/** Another arrow function. */
const internalHelper = (x: number): number => {
    return x * 2;
};

/** An async function. */
export async function loadConfig(path: string): Promise<Config> {
    const content = await fs.promises.readFile(path, 'utf-8');
    return JSON.parse(content);
}
