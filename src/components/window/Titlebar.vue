<template>
	<div data-tauri-drag-region class="titlebar">
		<div class="logo">
			<div class="w-6 h-6 flex shrink-0 p-1 items-center justify-center">
				<img src="/tauri.svg" alt="Logo" />
			</div>
			<span class="text-xs">Rapid Image Processor</span>
		</div>

		<div class="buttons">
			<div class="control-button" @click="appWindow.minimize()">
				<span>_</span>
			</div>
			<div class="control-button" @click="appWindow.toggleMaximize()">
				<span>□</span>
			</div>
			<div class="control-button hover:!bg-red-500" @click="appWindow.close()">
				<span>x</span>
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
	@apply bg-zinc-800 flex items-center justify-between select-none;
}

.logo {
	@apply flex items-center gap-2 h-8 pl-3 pointer-events-none;
}

.buttons {
	@apply flex items-center;
}

.control-button {
	@apply hover:bg-zinc-700 px-3 py-2 cursor-pointer transition-all starting:duration-300;
}
</style>
