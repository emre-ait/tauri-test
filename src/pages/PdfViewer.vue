<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { PDFDocument } from 'pdf-lib'
import { open } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'

interface ImageData {
	data: Uint8Array
	type: 'png' | 'jpg'
	url: string
	position: { x: number; y: number }
	rotation: number
	scale: number
	width: number // in cm
	height: number // in cm
	isResizing: boolean
}

const pdfDoc = ref<PDFDocument | null>(null)
const images = ref<ImageData[]>([])
const isDragging = ref(false)
const startPos = ref({ x: 0, y: 0 })
const draggedImageIndex = ref(-1)
const startDimensions = ref({ width: 0, height: 0 })

// Surface dimensions in cm
const surfaceWidth = ref(100)
const surfaceHeight = ref(100)

// Pixels per cm for display scaling
const PIXELS_PER_CM = 10

function updateSurfaceDimensions(width: number, height: number) {
	surfaceWidth.value = Math.max(10, Math.min(1000, width)) // min 10cm, max 1000cm
	surfaceHeight.value = Math.max(10, Math.min(1000, height)) // min 10cm, max 1000cm
}

function cmToPixels(cm: number): number {
	return cm * PIXELS_PER_CM
}

function pixelsToCm(pixels: number): number {
	return pixels / PIXELS_PER_CM
}

// Initialize PDF document
async function initPDF() {
	pdfDoc.value = await PDFDocument.create()
}

initPDF()

function handleCornerMouseDown(e: MouseEvent, index: number) {
	e.stopPropagation()
	const image = images.value[index]
	image.isResizing = true
	draggedImageIndex.value = index
	startPos.value = {
		x: e.clientX,
		y: e.clientY,
	}
	startDimensions.value = {
		width: image.width,
		height: image.height,
	}
}

function handleMouseMove(e: MouseEvent) {
	if (draggedImageIndex.value === -1) return
	const image = images.value[draggedImageIndex.value]

	if (image.isResizing) {
		const deltaX = pixelsToCm(e.clientX - startPos.value.x)
		const deltaY = pixelsToCm(e.clientY - startPos.value.y)
		image.width = Math.max(1, startDimensions.value.width + deltaX) // minimum 1cm
		image.height = Math.max(1, startDimensions.value.height + deltaY) // minimum 1cm
	} else if (isDragging.value) {
		const newX = pixelsToCm(e.clientX - startPos.value.x)
		const newY = pixelsToCm(e.clientY - startPos.value.y)

		// Constrain to surface boundaries
		image.position = {
			x: Math.max(0, Math.min(surfaceWidth.value - image.width, newX)),
			y: Math.max(0, Math.min(surfaceHeight.value - image.height, newY)),
		}
	}
}

function handleMouseUp() {
	if (draggedImageIndex.value !== -1) {
		images.value[draggedImageIndex.value].isResizing = false
	}
	isDragging.value = false
	draggedImageIndex.value = -1
}

function rotateLeft(index: number) {
	images.value[index].rotation = (images.value[index].rotation - 90) % 360
}

function rotateRight(index: number) {
	images.value[index].rotation = (images.value[index].rotation + 90) % 360
}

function handleMouseDown(e: MouseEvent, index: number) {
	isDragging.value = true
	draggedImageIndex.value = index
	const image = images.value[index]
	startPos.value = {
		x: e.clientX - cmToPixels(image.position.x),
		y: e.clientY - cmToPixels(image.position.y),
	}
}

async function addImage() {
	try {
		const selected = await open({
			multiple: false,
			filters: [
				{
					name: 'Image',
					extensions: ['png', 'jpg', 'jpeg'],
				},
			],
		})

		if (selected) {
			const imageBytes = await readFile(selected as string)
			const type = selected.toLowerCase().endsWith('.png') ? 'png' : 'jpg'
			const blob = new Blob([imageBytes], { type: `image/${type}` })
			const url = URL.createObjectURL(blob)

			// Create a temporary image to get dimensions
			const img = new Image()
			img.src = url
			await new Promise((resolve) => {
				img.onload = resolve
			})

			// Convert pixel dimensions to cm (assuming 96 DPI)
			const widthCm = (img.width / 96) * 2.54
			const heightCm = (img.height / 96) * 2.54

			images.value.push({
				data: imageBytes,
				type,
				url,
				position: { x: 0, y: 0 },
				rotation: 0,
				scale: 1,
				width: widthCm,
				height: heightCm,
				isResizing: false,
			})

			await updatePDF()
		}
	} catch (error) {
		console.error('Error processing image:', error)
	}
}

async function updatePDF() {
	if (!pdfDoc.value || images.value.length === 0) return

	// Clear existing pages
	const pageCount = pdfDoc.value.getPageCount()
	for (let i = 0; i < pageCount; i++) {
		pdfDoc.value.removePage(0)
	}

	// Create a single page for all images
	const page = pdfDoc.value.addPage([800, 1000])

	// Calculate grid layout
	const imagesPerRow = Math.ceil(Math.sqrt(images.value.length))
	const rows = Math.ceil(images.value.length / imagesPerRow)

	const pageWidth = page.getWidth()
	const pageHeight = page.getHeight()
	const padding = 20
	const availableWidth = pageWidth - padding * 2
	const availableHeight = pageHeight - padding * 2

	const imageWidth = availableWidth / imagesPerRow
	const imageHeight = availableHeight / rows

	// Add all images to the single page
	for (let i = 0; i < images.value.length; i++) {
		const imageData = images.value[i]
		const image =
			imageData.type === 'png'
				? await pdfDoc.value.embedPng(imageData.data)
				: await pdfDoc.value.embedJpg(imageData.data)

		// Calculate position in grid
		const row = Math.floor(i / imagesPerRow)
		const col = i % imagesPerRow

		// Calculate dimensions while maintaining aspect ratio
		const { width, height } = image.scale(1)
		const aspectRatio = width / height

		let scaledWidth = imageWidth - padding
		let scaledHeight = scaledWidth / aspectRatio

		if (scaledHeight > imageHeight - padding) {
			scaledHeight = imageHeight - padding
			scaledWidth = scaledHeight * aspectRatio
		}

		// Calculate position to center image in its grid cell
		const x = padding + col * imageWidth + (imageWidth - scaledWidth) / 2
		const y = pageHeight - padding - (row + 1) * imageHeight + (imageHeight - scaledHeight) / 2

		page.drawImage(image, {
			x,
			y,
			width: scaledWidth,
			height: scaledHeight,
		})
	}
}

async function downloadPDF() {
	if (!pdfDoc.value) return
	console.log('Downloading PDF')
	try {
		// Update PDF first
		await updatePDF()

		// Save the PDF
		const pdfBytes = await pdfDoc.value.save()

		// Create blob and download
		const blob = new Blob([pdfBytes], { type: 'application/pdf' })
		const url = URL.createObjectURL(blob)

		const link = document.createElement('a')
		link.href = url
		link.download = 'images.pdf'
		document.body.appendChild(link)
		link.click()
		document.body.removeChild(link)
		URL.revokeObjectURL(url)

		console.log('PDF downloaded')
		//see size of pdf and dimensions
		console.log('PDF size:', pdfBytes.byteLength)
		console.log(
			'PDF dimensions:',
			pdfDoc.value.getPageCount(),
			pdfDoc.value.getPage(0).getWidth(),
			pdfDoc.value.getPage(0).getHeight(),
		)
	} catch (error) {
		console.error('Error downloading PDF:', error)
	} finally {
		console.log('PDF downloaded')
	}
}

onUnmounted(() => {
	// Cleanup image URLs
	images.value.forEach((img) => URL.revokeObjectURL(img.url))
})
</script>

<template>
	<div class="p-4 h-screen flex flex-col">
		<div class="toolbar mb-4 flex items-center gap-4 bg-gray-800 p-4 rounded-lg">
			<button
				class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
				@click="addImage"
			>
				Add Image
			</button>

			<button
				v-if="images.length > 0"
				class="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 transition-colors"
				@click="downloadPDF"
			>
				Download PDF
			</button>

			<div class="flex items-center gap-2 ml-4">
				<span class="text-white">Surface:</span>
				<input
					v-model.number="surfaceWidth"
					type="number"
					class="w-16 px-2 py-1 bg-gray-700 text-white rounded"
					min="10"
					max="1000"
					@input="updateSurfaceDimensions(surfaceWidth, surfaceHeight)"
				/>
				<span class="text-white">×</span>
				<input
					v-model.number="surfaceHeight"
					type="number"
					class="w-16 px-2 py-1 bg-gray-700 text-white rounded"
					min="10"
					max="1000"
					@input="updateSurfaceDimensions(surfaceWidth, surfaceHeight)"
				/>
				<span class="text-white">cm</span>
			</div>
		</div>

		<div
			class="viewer bg-gray-900 rounded-lg overflow-hidden relative"
			:style="{
				width: cmToPixels(surfaceWidth) + 'px',
				height: cmToPixels(surfaceHeight) + 'px',
				minHeight: cmToPixels(10) + 'px',
			}"
			@mousemove="handleMouseMove"
			@mouseup="handleMouseUp"
			@mouseleave="handleMouseUp"
		>
			<template v-if="images.length > 0">
				<div
					v-for="(image, index) in images"
					:key="index"
					class="absolute"
					:style="{
						transform: `translate(${cmToPixels(image.position.x)}px, ${cmToPixels(image.position.y)}px) rotate(${image.rotation}deg)`,
						width: `${cmToPixels(image.width)}px`,
						height: `${cmToPixels(image.height)}px`,
						cursor: isDragging && draggedImageIndex === index ? 'grabbing' : 'grab',
						transition: isDragging && draggedImageIndex === index ? 'none' : 'transform 0.2s ease',
					}"
					@mousedown="(e) => handleMouseDown(e, index)"
				>
					<div class="relative group h-full w-full">
						<div
							class="w-full h-full"
							:style="{
								backgroundImage: `url(${image.url})`,
								backgroundSize: `${cmToPixels(5)}px ${cmToPixels(5)}px`, // 5cm × 5cm tiles
								backgroundRepeat: 'repeat',
							}"
						></div>
						<div
							class="absolute top-0 right-0 opacity-0 group-hover:opacity-100 transition-opacity bg-gray-800 rounded p-1 flex gap-1"
						>
							<button
								class="p-1 text-white hover:bg-gray-700 rounded"
								title="Rotate Left"
								@click.stop="rotateLeft(index)"
							>
								↺
							</button>
							<button
								class="p-1 text-white hover:bg-gray-700 rounded"
								title="Rotate Right"
								@click.stop="rotateRight(index)"
							>
								↻
							</button>
						</div>
						<div class="absolute bottom-0 left-0 bg-black bg-opacity-50 text-white text-xs px-1 rounded">
							{{ Math.round(image.width * 10) / 10 }}cm × {{ Math.round(image.height * 10) / 10 }}cm
						</div>
						<div
							class="absolute bottom-0 right-0 w-4 h-4 bg-white opacity-50 hover:opacity-100 cursor-se-resize"
							@mousedown.stop="(e) => handleCornerMouseDown(e, index)"
						></div>
					</div>
				</div>
			</template>
			<div v-else class="h-full flex items-center justify-center text-gray-400">Add images to create a PDF</div>
		</div>
	</div>
</template>

<style scoped>
.viewer {
	background-image: linear-gradient(45deg, #2c2c2c 25%, transparent 25%),
		linear-gradient(-45deg, #2c2c2c 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #2c2c2c 75%),
		linear-gradient(-45deg, transparent 75%, #2c2c2c 75%);
	background-size: 20px 20px;
	background-position:
		0 0,
		0 10px,
		10px -10px,
		-10px 0px;
	margin: 0 auto;
	max-width: 100%;
	max-height: calc(100vh - 120px); /* Account for toolbar and padding */
	overflow: auto;
	display: inline-block; /* This will make it fit content */
}
</style>
