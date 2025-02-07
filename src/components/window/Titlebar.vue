<template>
	<div data-tauri-drag-region class="titlebar">
		<div class="logo">
			<div class="w-6 h-6 flex shrink-0 p-1 items-center justify-center">
				<img src="/tauri.svg" alt="Logo" />
			</div>
			<span class="text-xs text-slate-50">Rapid Image Processor</span>
		</div>

		<div class="buttons">
			<div class="control-button text-slate-50" @click="appWindow.minimize()">
				<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
				</svg>
			</div>
			<div class="control-button text-slate-50" @click="appWindow.toggleMaximize()">
				<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
				</svg>
			</div>
			<div class="control-button text-slate-50 hover:!bg-red-500" @click="appWindow.close()">
				<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()
</script>

<style scoped>
@import "tailwindcss/theme";

.titlebar {
	@apply bg-zinc-800 flex items-center justify-between select-none fixed top-0 left-0 right-0 z-50;
}

.logo {
	@apply flex items-center gap-2 h-8 pl-3 pointer-events-none;
}

.buttons {
	@apply flex items-center;
}

.control-button {
	@apply hover:bg-zinc-700 px-3 py-2 cursor-pointer transition-all duration-200 flex items-center justify-center;
}
</style>
