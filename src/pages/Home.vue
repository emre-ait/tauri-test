<template>
	<div class="flex flex-col items-center gap-4 min-h-screen bg-gray-900 text-gray-100 p-6">
		<h1 class="text-3xl font-bold mb-4 text-blue-400">Welcome Home</h1>

		<!-- Navigation Buttons -->
		<div class="flex gap-4 mb-8">
			<router-link
				to="/snake"
				class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors shadow-lg hover:shadow-green-500/20"
			>
				Play Snake
			</router-link>
			<router-link
				to="/pdf-viewer"
				class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors shadow-lg hover:shadow-blue-500/20"
			>
				PDF Viewer
			</router-link>
		</div>

		<!-- Image Processing Section -->
		<div
			class="w-full max-w-md p-6 bg-gray-800 rounded-lg shadow-lg shadow-purple-500/10 mb-8 border border-gray-700"
		>
			<h2 class="text-xl font-bold mb-4 text-purple-400">Image Processing</h2>
			<div class="flex flex-col gap-4">
				<input
					type="file"
					accept="image/*"
					class="block w-full text-sm text-gray-300 file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:font-semibold file:bg-purple-600 file:text-white hover:file:bg-purple-700 file:cursor-pointer file:transition-colors cursor-pointer"
					@change="handleImageUpload"
				/>
				<div v-if="imageError" class="text-red-400 bg-red-900/20 p-3 rounded">
					{{ imageError }}
				</div>
				<div v-if="originalImage" class="border border-gray-700 rounded p-4 bg-gray-850">
					<h3 class="font-bold mb-2 text-blue-400">Original Image:</h3>
					<img :src="originalImage" class="max-w-full h-auto rounded" />
				</div>
				<div v-if="processedImage" class="border border-gray-700 rounded p-4 bg-gray-850">
					<h3 class="font-bold mb-2 text-green-400">Processed Image:</h3>
					<img :src="processedImage" class="max-w-full h-auto rounded" />
				</div>
			</div>
		</div>

		<!-- MIF Reader Section -->
		<div class="w-full max-w-md p-6 bg-gray-800 rounded-lg shadow-lg shadow-green-500/10 mb-8 border border-gray-700">
			<h2 class="text-xl font-bold mb-4 text-green-400">MIF Reader</h2>
			<div class="flex flex-col gap-4">
				<input
					type="file"
					accept=".mif"
					class="block w-full text-sm text-gray-300 file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:font-semibold file:bg-green-600 file:text-white hover:file:bg-green-700 file:cursor-pointer file:transition-colors cursor-pointer"
					@change="handleMifUpload"
				/>
				<div class="flex gap-2">
					<input
						v-model="layerIndex"
						type="number"
						min="0"
						placeholder="Layer Index"
						class="px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:border-green-500 focus:outline-none focus:ring-2 focus:ring-green-500/20 w-1/3"
					/>
					<input
						v-model="variantIndex"
						type="number"
						min="0"
						placeholder="Variant Index"
						class="px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:border-green-500 focus:outline-none focus:ring-2 focus:ring-green-500/20 w-1/3"
					/>
					<input
						v-model="scale"
						type="number"
						min="1"
						placeholder="Scale"
						class="px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:border-green-500 focus:outline-none focus:ring-2 focus:ring-green-500/20 w-1/3"
					/>
				</div>
				<div v-if="mifError" class="text-red-400 bg-red-900/20 p-3 rounded">
					{{ mifError }}
				</div>
				<div v-if="mifImage" class="border border-gray-700 rounded p-4 bg-gray-850">
					<h3 class="font-bold mb-2 text-green-400">MIF Image:</h3>
					<img :src="mifImage" class="max-w-full h-auto rounded" />
				</div>
			</div>
		</div>

		<!-- Calculator Section -->
		<div class="w-full max-w-md p-6 bg-gray-800 rounded-lg shadow-lg shadow-blue-500/10 border border-gray-700">
			<h2 class="text-xl font-bold mb-4 text-blue-400">Python Calculator</h2>
			<div class="flex flex-col gap-4">
				<input
					v-model="num1"
					type="number"
					placeholder="First Number"
					class="px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
				/>
				<input
					v-model="num2"
					type="number"
					placeholder="Second Number"
					class="px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
				/>
				<div class="grid grid-cols-2 gap-2">
					<button
						class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors shadow-lg hover:shadow-blue-500/20"
						@click="calculate('add')"
					>
						Add (+)
					</button>
					<button
						class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors shadow-lg hover:shadow-green-500/20"
						@click="calculate('subtract')"
					>
						Subtract (-)
					</button>
					<button
						class="px-4 py-2 bg-yellow-600 text-white rounded hover:bg-yellow-700 transition-colors shadow-lg hover:shadow-yellow-500/20"
						@click="calculate('multiply')"
					>
						Multiply (×)
					</button>
					<button
						class="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 transition-colors shadow-lg hover:shadow-red-500/20"
						@click="calculate('divide')"
					>
						Divide (÷)
					</button>
				</div>
				<div v-if="result" class="mt-4 p-4 bg-gray-700 rounded text-center border border-gray-600">
					Result: {{ result }}
				</div>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const num1 = ref('')
const num2 = ref('')
const result = ref('')
const originalImage = ref('')
const processedImage = ref('')
const imageError = ref('')
const mifImage = ref('')
const mifError = ref('')
const layerIndex = ref(0)
const variantIndex = ref(0)
const scale = ref(1)

async function handleImageUpload(event: Event) {
	const file = (event.target as HTMLInputElement).files?.[0]
	if (!file) {
		imageError.value = 'No file selected'
		return
	}

	try {
		// Reset states
		imageError.value = ''
		processedImage.value = ''

		// Read and display original image
		const reader = new FileReader()
		reader.onload = async (e) => {
			originalImage.value = e.target?.result as string

			// Process image with Python
			try {
				processedImage.value = await invoke('process_image', {
					imageData: originalImage.value,
				})
			} catch (error) {
				imageError.value = `Processing error: ${error}`
			}
		}
		reader.readAsDataURL(file)
	} catch (error) {
		imageError.value = `Upload error: ${error}`
	}
}

async function handleMifUpload(event: Event) {
	const file = (event.target as HTMLInputElement).files?.[0]
	if (!file) {
		mifError.value = 'No file selected'
		return
	}

	try {
		// Reset states
		mifError.value = ''
		mifImage.value = ''

		// Get file path using Tauri dialog
		const filePath = await open({
			multiple: false,
			filters: [{
				name: 'MIF Files',
				extensions: ['mif']
			}]
		})

		if (!filePath || typeof filePath !== 'string') {
			mifError.value = 'No file selected'
			return
		}

		// Process MIF file with Python
		try {
			mifImage.value = await invoke('mif_reader', {
				filePath,
				layerIndex: parseInt(layerIndex.value.toString()),
				variantIndex: parseInt(variantIndex.value.toString()),
				scale: parseInt(scale.value.toString())
			})

			if (mifImage.value.startsWith('Error:')) {
				mifError.value = mifImage.value
				mifImage.value = ''
			}
		} catch (error) {
			mifError.value = `Processing error: ${error}`
		}
	} catch (error) {
		mifError.value = `Upload error: ${error}`
	}
}

async function calculate(operation: string) {
	try {
		if (num1.value === '' || num2.value === '') {
			result.value = 'Please enter both numbers'
			return
		}

		const a = parseFloat(num1.value)
		const b = parseFloat(num2.value)

		result.value = await invoke('calculate', {
			operation,
			a,
			b,
		})
	} catch (error) {
		result.value = `Error: ${error}`
	}
}
</script>
