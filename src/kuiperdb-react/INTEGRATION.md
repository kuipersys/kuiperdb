# @kuiperdb/react Package

Successfully created a new React-specific package for KuiperDb!

## Package Structure

```
@kuiperdb/client (TypeScript Client)
    ↓
@kuiperdb/react (React Components & Hooks)
    ↓
kuiperdb-ui (UI Application)
```

## What's Included

### @kuiperdb/react
- **Location**: `src/kuiperdb-react/`
- **Exports**: 
  - `KuiperDbProvider` - React Context Provider
  - `useKuiperDb()` - Hook to access KuiperDb client
- **Dependencies**:
  - `@kuiperdb/client` (peer: `react`)

## Usage

### In kuiperdb-ui

```tsx
import { KuiperDbProvider, useKuiperDb } from '@kuiperdb/react';

// In main.tsx
<KuiperDbProvider baseURL="/">
  <App />
</KuiperDbProvider>

// In any component
function MyComponent() {
  const { client } = useKuiperDb();
  // ... use client
}
```

### In External Projects

```bash
npm install @kuiperdb/react @kuiperdb/client
```

```tsx
import { KuiperDbProvider, useKuiperDb } from '@kuiperdb/react';
// Same usage as above
```

## Benefits

1. **Separation of Concerns**: Client logic separate from React bindings
2. **Reusability**: Can use `@kuiperdb/client` in Node.js, `@kuiperdb/react` in React apps
3. **Tree Shaking**: Only import what you need
4. **Type Safety**: Full TypeScript support across all packages
5. **Independent Versioning**: Each package can be versioned separately

## Files Modified

- ✅ Created `src/kuiperdb-react/` package
- ✅ Updated `src/kuiperdb-ui/package.json` to use `@kuiperdb/react`
- ✅ Updated `src/kuiperdb-ui/src/main.tsx` to import from `@kuiperdb/react`
- ✅ Updated `Dockerfile` to build `@kuiperdb/react` before UI
- ✅ Kept backwards compatibility in `src/kuiperdb-ui/src/providers/index.ts`

## Build Order

1. `@kuiperdb/client` (TypeScript client)
2. `@kuiperdb/react` (React bindings)
3. `kuiperdb-ui` (UI application)
