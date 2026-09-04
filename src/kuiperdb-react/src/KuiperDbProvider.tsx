import { createContext, useContext, createElement } from 'react';
import type { ReactNode } from 'react';
import { KuiperDbClient, createClient } from '@kuiperdb/client';

interface KuiperDbContextValue {
  client: KuiperDbClient;
}

const KuiperDbContext = createContext<KuiperDbContextValue | undefined>(undefined);

interface KuiperDbProviderProps {
  children: ReactNode;
  baseURL?: string;
  timeout?: number;
}

/**
 * KuiperDbProvider - React Context Provider for KuiperDb Client
 * 
 * Provides a configured KuiperDb client instance to all child components.
 * 
 * @example
 * ```tsx
 * import { KuiperDbProvider } from '@kuiperdb/react';
 * 
 * function App() {
 *   return (
 *     <KuiperDbProvider baseURL="http://localhost:17001">
 *       <YourComponents />
 *     </KuiperDbProvider>
 *   );
 * }
 * ```
 */
export function KuiperDbProvider({ 
  children, 
  baseURL = '/',
  timeout = 30000 
}: KuiperDbProviderProps) {
  const client = createClient({ baseURL, timeout });

  return createElement(
    KuiperDbContext.Provider,
    { value: { client } },
    children
  );
}

/**
 * useKuiperDb - Hook to access KuiperDb client
 * 
 * Returns the configured KuiperDb client instance from context.
 * Must be used within a KuiperDbProvider.
 * 
 * @throws {Error} If used outside of KuiperDbProvider
 * 
 * @example
 * ```tsx
 * import { useKuiperDb } from '@kuiperdb/react';
 * 
 * function MyComponent() {
 *   const { client } = useKuiperDb();
 *   
 *   const results = await client.search({
 *     space: 'semantic',
 *     vector: [1, 0, 0],
 *   });
 * }
 * ```
 */
export function useKuiperDb(): KuiperDbContextValue {
  const context = useContext(KuiperDbContext);
  
  if (!context) {
    throw new Error('useKuiperDb must be used within a KuiperDbProvider');
  }
  
  return context;
}
