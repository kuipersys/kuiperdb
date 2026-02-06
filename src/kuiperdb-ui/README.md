# KuiperDb Explorer UI

A React-based UI for exploring the KuiperDb database contents.

## Features

- Tree view navigation: Databases → Tables → Documents (roots) → Children
- Document content viewer
- Health status monitoring  
- Auto-refresh capability

## Tech Stack

- **Vite** - Fast build tool and dev server
- **React 18** - UI framework
- **TypeScript** - Type safety
- **Mantine** - UI component library
- **Tabler Icons** - Icon library
- **Axios** - HTTP client
- **TanStack Query** - Data fetching

## Prerequisites

- Node.js 18+ and npm
- KuiperDb server running on `http://localhost:8080`

## Installation

```bash
cd kuiperdb-ui
npm install
```

## Development

Start the development server:

```bash
npm run dev
```

The UI will be available at `http://localhost:5173` (or the next available port).

## Building for Production

```bash
npm run build
```

The built files will be in the `dist` directory.

## Project Structure

```
kuiperdb-ui/
├── src/
│   ├── api/           # API client and types
│   ├── components/    # React components
│   ├── hooks/         # Custom React hooks
│   ├── types/         # TypeScript type definitions
│   ├── utils/         # Utility functions
│   ├── App.tsx        # Main application component
│   └── main.tsx       # Application entry point
├── public/            # Static assets
└── package.json
```

## API Endpoints Used

- `GET /health` - Server health check
- `GET /db` - List databases
- `GET /db/{db_name}/tables` - List tables in a database
- `GET /db/{db_name}/{table_name}/documents` - List root documents in a table
- `GET /db/{db_name}/{table_name}/{doc_id}` - Get document details
- `GET /db/{db_name}/documents/{doc_id}/relations` - Get document relations

## Usage

1. Ensure the KuiperDb server is running
2. Start the UI development server
3. Open your browser to the provided URL
4. Click on databases in the tree to expand and view tables
5. Click on tables to see root documents
6. Click on documents to view their full content

## Notes

- The tree view currently shows root documents (documents without a parent_id)
- Document relations and child documents are visible in the content viewer
- Graphs are not yet implemented (planned future feature)
