import { defineConfig } from 'vitest/config'

// 3D casts take a few seconds each, longer when every test file runs at once.
export default defineConfig({ test: { testTimeout: 30_000 } })
