/**
 * TransportProvider — React context that initializes a Transport instance
 * and exposes it to all child components.
 *
 * Wrapping the app in <TransportProvider> ensures a single transport instance
 * is created for the lifetime of the application.
 */
import { createContext, useMemo, type ReactNode } from 'react';
import { createTransport, type Transport } from './transport';

const TransportContext = createContext<Transport | null>(null);

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
