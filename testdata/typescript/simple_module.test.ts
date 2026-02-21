import { describe, it, expect } from 'vitest';
import { createLogger, DataService } from './simple_module';

describe('createLogger', () => {
    it('should create a logger', () => {
        const logger = createLogger('test');
        expect(logger).toBeDefined();
    });
});

describe('DataService', () => {
    it('should create a service', () => {
        const service = new DataService('test');
        expect(service.getName()).toBe('test');
    });
});
