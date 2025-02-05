<template>
	<div class="min-h-screen bg-gray-900 text-slate-50">
		<Titlebar />
		<div class="pt-8">
			<router-view></router-view>
		</div>
	</div>
</template>

<script setup lang="ts">
import Titlebar from './components/window/Titlebar.vue'
</script>

<style>
/* Global styles */
:root {
  color-scheme: dark;
}

</style>
