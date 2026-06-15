/**
 * TransportProvider — React context that initializes a Transport instance
 * and exposes it to all child components via the useTransport() hook.
 *
 * Wrapping the app in <TransportProvider> ensures every component can call
 * transport.invoke() and transport.listen() without importing Tauri directly.
 */
import { createContext, useContext, useMemo, type ReactNode } from 'react';
import { createTransport, type Transport } from './transport';

const TransportContext = createContext<Transport | null>(null);

/**
 * Hook for consuming the transport instance from any component.
 * Must be called within a <TransportProvider> tree.
 */
export function useTransport(): Transport {
  const transport = useContext(TransportContext);
  if (!transport) {
    throw new Error('useTransport must be used within a <TransportProvider>');
  }
  return transport;
}

interface TransportProviderProps {
  children: ReactNode;
}

/**
 * Provider component — creates a single Transport instance (TauriTransport
 * or HttpTransport depending on runtime environment) and provides it via context.
 */
export function TransportProvider({ children }: TransportProviderProps) {
  const transport = useMemo(() => createTransport(), []);

  return (
    <TransportContext.Provider value={transport}>
      {children}
    </TransportContext.Provider>
  );
}
