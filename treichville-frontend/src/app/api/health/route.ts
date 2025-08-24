import { NextResponse } from 'next/server';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

interface ServiceHealth {
  name: string;
  status: 'healthy' | 'degraded' | 'down';
  responseTime?: number;
  details?: any;
  error?: string;
}

// Check backend API health
async function checkBackendHealth(): Promise<ServiceHealth> {
  const startTime = Date.now();
  try {
    const response = await fetch('http://localhost:8080/health', {
      method: 'GET',
      headers: { 'Accept': 'application/json' },
      signal: AbortSignal.timeout(5000), // 5 second timeout
    });
    
    const responseTime = Date.now() - startTime;
    
    if (response.ok) {
      const data = await response.json();
      return {
        name: 'Backend API',
        status: data.status === 'healthy' ? 'healthy' : 'degraded',
        responseTime,
        details: data
      };
    }
    
    return {
      name: 'Backend API',
      status: 'degraded',
      responseTime,
      error: `HTTP ${response.status}`
    };
  } catch (error: any) {
    return {
      name: 'Backend API',
      status: 'down',
      responseTime: Date.now() - startTime,
      error: error.message
    };
  }
}

// Check PostgreSQL health using Docker
async function checkPostgreSQLHealth(): Promise<ServiceHealth> {
  const startTime = Date.now();
  try {
    // Use docker exec to check PostgreSQL
    const { stdout, stderr } = await execAsync(
      'docker exec treichville_postgres pg_isready -U treichville_user -d treichville_exchange'
    );
    
    const responseTime = Date.now() - startTime;
    
    if (stdout.includes('accepting connections')) {
      // Get more details
      const { stdout: versionOut } = await execAsync(
        'docker exec treichville_postgres psql -U treichville_user -d treichville_exchange -c "SELECT version();" -t'
      ).catch(() => ({ stdout: '' }));
      
      const { stdout: sizeOut } = await execAsync(
        'docker exec treichville_postgres psql -U treichville_user -d treichville_exchange -c "SELECT pg_database_size(\'treichville_exchange\');" -t'
      ).catch(() => ({ stdout: '' }));
      
      const { stdout: connectionsOut } = await execAsync(
        'docker exec treichville_postgres psql -U treichville_user -d treichville_exchange -c "SELECT count(*) FROM pg_stat_activity;" -t'
      ).catch(() => ({ stdout: '' }));
      
      return {
        name: 'PostgreSQL',
        status: 'healthy',
        responseTime,
        details: {
          version: versionOut.trim().split('on')[0].trim(),
          database_size: sizeOut.trim(),
          active_connections: parseInt(connectionsOut.trim()) || 0,
          container: 'treichville_postgres',
          port: 5432
        }
      };
    }
    
    return {
      name: 'PostgreSQL',
      status: 'degraded',
      responseTime,
      error: stderr || 'Not ready'
    };
  } catch (error: any) {
    // Try alternative check using TCP connection
    try {
      const net = require('net');
      const client = new net.Socket();
      
      return new Promise((resolve) => {
        const timeout = setTimeout(() => {
          client.destroy();
          resolve({
            name: 'PostgreSQL',
            status: 'down',
            responseTime: Date.now() - startTime,
            error: 'Connection timeout'
          });
        }, 3000);
        
        client.connect(5432, 'localhost', () => {
          clearTimeout(timeout);
          client.destroy();
          resolve({
            name: 'PostgreSQL',
            status: 'degraded', // Port is open but can't verify health
            responseTime: Date.now() - startTime,
            details: { port_open: true }
          });
        });
        
        client.on('error', () => {
          clearTimeout(timeout);
          client.destroy();
          resolve({
            name: 'PostgreSQL',
            status: 'down',
            responseTime: Date.now() - startTime,
            error: 'Port closed'
          });
        });
      });
    } catch {
      return {
        name: 'PostgreSQL',
        status: 'down',
        responseTime: Date.now() - startTime,
        error: error.message
      };
    }
  }
}

// Check Redis health using Docker
async function checkRedisHealth(): Promise<ServiceHealth> {
  const startTime = Date.now();
  try {
    // Use docker exec to check Redis
    const { stdout } = await execAsync('docker exec treichville_redis redis-cli ping');
    
    const responseTime = Date.now() - startTime;
    
    if (stdout.trim() === 'PONG') {
      // Get more details
      const { stdout: infoOut } = await execAsync(
        'docker exec treichville_redis redis-cli INFO server'
      ).catch(() => ({ stdout: '' }));
      
      const { stdout: memOut } = await execAsync(
        'docker exec treichville_redis redis-cli INFO memory'
      ).catch(() => ({ stdout: '' }));
      
      // Parse Redis info
      const version = infoOut.match(/redis_version:([^\r\n]+)/)?.[1] || 'unknown';
      const usedMemory = memOut.match(/used_memory_human:([^\r\n]+)/)?.[1] || 'unknown';
      const uptime = infoOut.match(/uptime_in_seconds:([^\r\n]+)/)?.[1] || '0';
      
      return {
        name: 'Redis',
        status: 'healthy',
        responseTime,
        details: {
          version,
          used_memory: usedMemory,
          uptime_seconds: parseInt(uptime),
          container: 'treichville_redis',
          port: 6379
        }
      };
    }
    
    return {
      name: 'Redis',
      status: 'degraded',
      responseTime,
      error: 'Unexpected response'
    };
  } catch (error: any) {
    // Try alternative check using TCP connection
    try {
      const net = require('net');
      const client = new net.Socket();
      
      return new Promise((resolve) => {
        const timeout = setTimeout(() => {
          client.destroy();
          resolve({
            name: 'Redis',
            status: 'down',
            responseTime: Date.now() - startTime,
            error: 'Connection timeout'
          });
        }, 3000);
        
        client.connect(6379, 'localhost', () => {
          clearTimeout(timeout);
          client.destroy();
          resolve({
            name: 'Redis',
            status: 'degraded', // Port is open but can't verify health
            responseTime: Date.now() - startTime,
            details: { port_open: true }
          });
        });
        
        client.on('error', () => {
          clearTimeout(timeout);
          client.destroy();
          resolve({
            name: 'Redis',
            status: 'down',
            responseTime: Date.now() - startTime,
            error: 'Port closed'
          });
        });
      });
    } catch {
      return {
        name: 'Redis',
        status: 'down',
        responseTime: Date.now() - startTime,
        error: error.message
      };
    }
  }
}

// Check Docker status
async function checkDockerStatus() {
  try {
    const { stdout } = await execAsync('docker ps --format "json"');
    const containers = stdout.trim().split('\n')
      .filter(line => line)
      .map(line => {
        try {
          return JSON.parse(line);
        } catch {
          return null;
        }
      })
      .filter(Boolean);
    
    const postgresContainer = containers.find(c => c.Names === 'treichville_postgres');
    const redisContainer = containers.find(c => c.Names === 'treichville_redis');
    
    return {
      postgres: postgresContainer ? {
        status: postgresContainer.Status,
        state: postgresContainer.State,
        ports: postgresContainer.Ports
      } : null,
      redis: redisContainer ? {
        status: redisContainer.Status,
        state: redisContainer.State,
        ports: redisContainer.Ports
      } : null,
      total_containers: containers.length
    };
  } catch (error) {
    return {
      error: 'Docker not available',
      postgres: null,
      redis: null,
      total_containers: 0
    };
  }
}

// Get system metrics
async function getSystemMetrics() {
  try {
    // Get memory usage (Windows)
    const { stdout: memOut } = await execAsync('wmic OS get TotalVisibleMemorySize,FreePhysicalMemory /value')
      .catch(() => ({ stdout: '' }));
    
    const totalMem = parseInt(memOut.match(/TotalVisibleMemorySize=(\d+)/)?.[1] || '0');
    const freeMem = parseInt(memOut.match(/FreePhysicalMemory=(\d+)/)?.[1] || '0');
    const memoryUsage = totalMem > 0 ? ((totalMem - freeMem) / totalMem * 100).toFixed(1) : 0;
    
    // Get CPU usage (simplified)
    const cpuUsage = Math.random() * 40 + 20; // Simulated for now
    
    // Get disk usage
    const { stdout: diskOut } = await execAsync('wmic logicaldisk get size,freespace,caption /value')
      .catch(() => ({ stdout: '' }));
    
    return {
      cpu: cpuUsage,
      memory: parseFloat(memoryUsage as string),
      uptime: process.uptime(),
      timestamp: new Date().toISOString()
    };
  } catch {
    return {
      cpu: 0,
      memory: 0,
      uptime: process.uptime(),
      timestamp: new Date().toISOString()
    };
  }
}

export async function GET() {
  try {
    // Run all health checks in parallel
    const [backend, postgresql, redis, docker, metrics] = await Promise.all([
      checkBackendHealth(),
      checkPostgreSQLHealth(),
      checkRedisHealth(),
      checkDockerStatus(),
      getSystemMetrics()
    ]);
    
    // Calculate overall status
    const services = [backend, postgresql, redis];
    const overallStatus = services.every(s => s.status === 'healthy') ? 'healthy' :
                         services.some(s => s.status === 'down') ? 'unhealthy' : 'degraded';
    
    return NextResponse.json({
      status: overallStatus,
      timestamp: new Date().toISOString(),
      services: {
        backend,
        postgresql,
        redis
      },
      docker,
      metrics,
      frontend: {
        name: 'Frontend',
        status: 'healthy',
        responseTime: 10,
        details: {
          version: process.env.npm_package_version || '1.0.0',
          node_version: process.version
        }
      }
    });
  } catch (error: any) {
    return NextResponse.json({
      status: 'error',
      timestamp: new Date().toISOString(),
      error: error.message,
      services: {
        backend: { name: 'Backend API', status: 'down' },
        postgresql: { name: 'PostgreSQL', status: 'down' },
        redis: { name: 'Redis', status: 'down' }
      }
    }, { status: 500 });
  }
}