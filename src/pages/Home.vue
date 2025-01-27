<template>
	<div class="flex flex-col items-center gap-4 p-4">
		<h1 class="text-2xl font-bold mb-4">Welcome Home</h1>
		<div class="flex gap-4">
			<router-link
				to="/snake"
				class="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 transition-colors"
			>
				Play Snake
			</router-link>
			<router-link
				to="/pdf-viewer"
				class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
			>
				PDF Viewer
			</router-link>
		</div>
	</div>
</template>
