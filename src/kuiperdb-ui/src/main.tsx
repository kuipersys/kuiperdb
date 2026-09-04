import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { Notifications } from '@mantine/notifications'
import { KuiperDbProvider } from '@kuiperdb/react'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <KuiperDbProvider baseURL={import.meta.env.VITE_KUIPERDB_URL ?? 'http://localhost:17001'}>
      <MantineProvider>
        <ModalsProvider>
          <Notifications position="top-right" />
          <App />
        </ModalsProvider>
      </MantineProvider>
    </KuiperDbProvider>
  </StrictMode>,
)
