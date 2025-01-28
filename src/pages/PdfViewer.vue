<script setup lang="ts">
import { ref, onUnmounted, onMounted } from 'vue'
import { PDFDocument } from 'pdf-lib'
import { open } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import ExifReader from 'exif-reader'

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
	filename: string
}

const pdfDoc = ref<PDFDocument | null>(null)
const images = ref<ImageData[]>([])
const isDragging = ref(false)
const startPos = ref({ x: 0, y: 0 })
const draggedImageIndex = ref(-1)
const startDimensions = ref({ width: 0, height: 0 })
const selectedImageIndex = ref(-1)
const zoom = ref(1)
const minZoom = 0.1
const maxZoom = 5

// Surface dimensions in cm
const surfaceWidth = ref(100)
const surfaceHeight = ref(100)

// Pixels per cm for display scaling
const PIXELS_PER_CM = 10

// Add these refs after other refs
const isPanning = ref(false)
const isSpacePressed = ref(false)
const viewerRef = ref<HTMLElement | null>(null)
const panStart = ref({ x: 0, y: 0 })
const isFileMenuOpen = ref(false)
const fileMenuRef = ref<HTMLElement | null>(null)
const arrangeGap = ref(1) // Default 1cm gap

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

	// Add these lines after initPDF()
	window.addEventListener('keydown', handleKeyDown)
	window.addEventListener('keyup', handleKeyUp)
}

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
		const deltaX = pixelsToCm(e.clientX - startPos.value.x) / zoom.value
		const deltaY = pixelsToCm(e.clientY - startPos.value.y) / zoom.value
		
		// Calculate new dimensions
		let newWidth = Math.max(1, startDimensions.value.width + deltaX)
		let newHeight = Math.max(1, startDimensions.value.height + deltaY)

		// Constrain width to surface bounds
		if (image.position.x + newWidth > surfaceWidth.value) {
			newWidth = surfaceWidth.value - image.position.x
		}

		// Constrain height to surface bounds
		if (image.position.y + newHeight > surfaceHeight.value) {
			newHeight = surfaceHeight.value - image.position.y
		}

		// Apply constrained dimensions
		image.width = Math.max(1, newWidth)
		image.height = Math.max(1, newHeight)
	} else if (isDragging.value) {
		const newX = pixelsToCm(e.clientX - startPos.value.x) / zoom.value
		const newY = pixelsToCm(e.clientY - startPos.value.y) / zoom.value

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
	selectedImageIndex.value = index
	const image = images.value[index]
	startPos.value = {
		x: e.clientX - cmToPixels(image.position.x) * zoom.value,
		y: e.clientY - cmToPixels(image.position.y) * zoom.value,
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
			
			const filename = (selected as string).split(/[/\\]/).pop() || ''

			// Create temporary image to get dimensions
			const img = new Image()
			img.src = url
			await new Promise((resolve) => {
				img.onload = resolve
			})

			// Try to get resolution from EXIF data
			let xResolution = 72 // Default resolution
			let yResolution = 72
			let resolutionUnit = 2 // 2 = inches, 3 = cm

			try {
				const exifData = ExifReader.load(imageBytes)
				if (exifData?.exif) {
					xResolution = exifData.exif.XResolution || xResolution
					yResolution = exifData.exif.YResolution || yResolution
					resolutionUnit = exifData.exif.ResolutionUnit || resolutionUnit
				}
			} catch (e) {
				console.warn('Could not read EXIF data, using default resolution')
			}

			// Convert to DPI if resolution unit is cm
			if (resolutionUnit === 3) {
				xResolution = xResolution * 2.54
				yResolution = yResolution * 2.54
			}

			// Convert pixel dimensions to cm
			const widthCm = (img.width / xResolution) * 2.54
			const heightCm = (img.height / yResolution) * 2.54

			// Only scale down if larger than surface
			let finalWidthCm = widthCm
			let finalHeightCm = heightCm

			if (widthCm > surfaceWidth.value || heightCm > surfaceHeight.value) {
				const scaleW = surfaceWidth.value / widthCm
				const scaleH = surfaceHeight.value / heightCm
				const scale = Math.min(scaleW, scaleH)
				finalWidthCm *= scale
				finalHeightCm *= scale
			}

			images.value.push({
				data: imageBytes,
				type,
				url,
				position: { x: 0, y: 0 },
				rotation: 0,
				scale: 1,
				width: finalWidthCm,
				height: finalHeightCm,
				isResizing: false,
				filename,
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

function handleWheel(e: WheelEvent) {
	if (e.ctrlKey || e.metaKey) {
		e.preventDefault()
		
		const container = viewerRef.value
		if (!container) return

		const rect = container.getBoundingClientRect()
		const containerCenterX = rect.left + rect.width / 2

		// Get mouse position relative to container center (x) and top (y)
		const mouseX = e.clientX - containerCenterX + container.scrollLeft
		const mouseY = e.clientY - rect.top + container.scrollTop

		// Calculate zoom
		const delta = -Math.sign(e.deltaY) * 0.1
		const newZoom = Math.max(minZoom, Math.min(maxZoom, zoom.value + delta))
		
		if (newZoom !== zoom.value) {
			// Calculate scaling factor
			const scale = newZoom / zoom.value
			
			// Update zoom
			zoom.value = newZoom

			// Adjust scroll position to keep mouse point fixed
			container.scrollLeft = mouseX * scale - (e.clientX - containerCenterX)
			container.scrollTop = mouseY * scale - (e.clientY - rect.top)
		}
	}
}

function handleKeyDown(e: KeyboardEvent) {
	if (e.code === 'Space') {
		e.preventDefault()
		isSpacePressed.value = true
	}

	// Add keyboard shortcuts
	if (!e.ctrlKey && !e.metaKey) {
		switch (e.code) {
			case 'KeyA':
				if (images.length > 0) {
					arrangeImages()
				}
				break
			case 'KeyD':
				if (selectedImageIndex.value !== -1) {
					duplicateImage(selectedImageIndex.value)
				}
				break
			case 'KeyL':
				if (selectedImageIndex.value !== -1) {
					rotateLeft(selectedImageIndex.value)
				}
				break
			case 'KeyR':
				if (selectedImageIndex.value !== -1) {
					rotateRight(selectedImageIndex.value)
				}
				break
			case 'KeyS':
				if (selectedImageIndex.value !== -1) {
					resetImageSize(selectedImageIndex.value)
				}
				break
			case 'KeyV':
				resetView()
				break
			case 'Delete':
				if (selectedImageIndex.value !== -1) {
					removeImage(selectedImageIndex.value)
				}
				break
			case 'KeyF':
				if (selectedImageIndex.value !== -1) {
					bringToFront(selectedImageIndex.value)
				}
				break
			case 'KeyB':
				if (selectedImageIndex.value !== -1) {
					sendToBack(selectedImageIndex.value)
				}
				break
		}
	}
}

function handleKeyUp(e: KeyboardEvent) {
	if (e.code === 'Space') {
		isSpacePressed.value = false
	}
}

function handleViewerMouseDown(e: MouseEvent) {
	// Middle mouse button (button 1) or left click + space
	if (e.button === 1 || (e.button === 0 && isSpacePressed.value)) {
		e.preventDefault()
		isPanning.value = true
		panStart.value = {
			x: e.clientX + (viewerRef.value?.scrollLeft || 0),
			y: e.clientY + (viewerRef.value?.scrollTop || 0),
		}
	}
}

function handleViewerMouseMove(e: MouseEvent) {
	if (isPanning.value && viewerRef.value) {
		viewerRef.value.scrollLeft = panStart.value.x - e.clientX
		viewerRef.value.scrollTop = panStart.value.y - e.clientY
	}
}

function handleViewerMouseUp() {
	isPanning.value = false
}

function removeImage(index: number) {
	// Revoke URL before removing
	URL.revokeObjectURL(images.value[index].url)
	images.value.splice(index, 1)
	selectedImageIndex.value = -1
}

function duplicateImage(index: number) {
	const original = images.value[index]
	const newUrl = URL.createObjectURL(new Blob([original.data], { type: `image/${original.type}` }))
	
	// Create copy with slight offset
	const copy: ImageData = {
		...original,
		url: newUrl,
		position: {
			x: original.position.x + 1,
			y: original.position.y + 1
		}
	}
	
	images.value.push(copy)
}

async function resetImageSize(index: number) {
	const image = images.value[index]
	
	// Create temporary image to get dimensions
	const img = new Image()
	img.src = image.url
	await new Promise((resolve) => {
		img.onload = resolve
	})

	// Try to get resolution from EXIF data
	let xResolution = 72 // Default resolution
	let yResolution = 72
	let resolutionUnit = 2 // 2 = inches, 3 = cm

	try {
		const exifData = ExifReader.load(image.data)
		if (exifData?.exif) {
			xResolution = exifData.exif.XResolution || xResolution
			yResolution = exifData.exif.YResolution || yResolution
			resolutionUnit = exifData.exif.ResolutionUnit || resolutionUnit
		}
	} catch (e) {
		console.warn('Could not read EXIF data, using default resolution')
	}

	// Convert to DPI if resolution unit is cm
	if (resolutionUnit === 3) {
		xResolution = xResolution * 2.54
		yResolution = yResolution * 2.54
	}

	// Convert pixel dimensions to cm
	const widthCm = (img.width / xResolution) * 2.54
	const heightCm = (img.height / yResolution) * 2.54

	// Only scale down if larger than surface
	let finalWidthCm = widthCm
	let finalHeightCm = heightCm

	if (widthCm > surfaceWidth.value || heightCm > surfaceHeight.value) {
		const scaleW = surfaceWidth.value / widthCm
		const scaleH = surfaceHeight.value / heightCm
		const scale = Math.min(scaleW, scaleH)
		finalWidthCm *= scale
		finalHeightCm *= scale
	}

	// Update image dimensions
	image.width = finalWidthCm
	image.height = finalHeightCm
}

function clearAllImages() {
	// Clean up URLs before removing
	images.value.forEach(img => URL.revokeObjectURL(img.url))
	images.value = []
	selectedImageIndex.value = -1
}

function resetView() {
	zoom.value = 1
	if (viewerRef.value) {
		viewerRef.value.scrollLeft = 0
		viewerRef.value.scrollTop = 0
	}
}

function bringToFront(index: number) {
	const image = images.value[index]
	images.value.splice(index, 1)
	images.value.push(image)
}

function sendToBack(index: number) {
	const image = images.value[index]
	images.value.splice(index, 1)
	images.value.unshift(image)
}

function newSurface() {
	if (images.value.length > 0) {
		if (!confirm('Are you sure you want to create a new surface? All unsaved changes will be lost.')) {
			return
		}
	}
	clearAllImages()
	surfaceWidth.value = 100
	surfaceHeight.value = 100
	zoom.value = 1
	if (viewerRef.value) {
		viewerRef.value.scrollLeft = 0
		viewerRef.value.scrollTop = 0
	}
}

async function saveProject() {
	try {
		const projectData = {
			surfaceWidth: surfaceWidth.value,
			surfaceHeight: surfaceHeight.value,
			images: images.value.map(img => ({
				...img,
				data: Array.from(img.data), // Convert Uint8Array to regular array for JSON
			})),
		}

		const blob = new Blob([JSON.stringify(projectData)], { type: 'application/json' })
		const url = URL.createObjectURL(blob)

		const link = document.createElement('a')
		link.href = url
		link.download = 'surface_project.json'
		document.body.appendChild(link)
		link.click()
		document.body.removeChild(link)
		URL.revokeObjectURL(url)
	} catch (error) {
		console.error('Error saving project:', error)
	}
}

async function loadProject() {
	try {
		const selected = await open({
			multiple: false,
			filters: [
				{
					name: 'Surface Project',
					extensions: ['json'],
				},
			],
		})

		if (selected) {
			const fileContent = await readFile(selected as string)
			const projectData = JSON.parse(new TextDecoder().decode(fileContent))

			// Clean up existing images
			clearAllImages()

			// Restore surface dimensions
			surfaceWidth.value = projectData.surfaceWidth
			surfaceHeight.value = projectData.surfaceHeight

			// Restore images
			for (const imgData of projectData.images) {
				const data = new Uint8Array(imgData.data)
				const blob = new Blob([data], { type: `image/${imgData.type}` })
				const url = URL.createObjectURL(blob)

				images.value.push({
					...imgData,
					data,
					url,
				})
			}
		}
	} catch (error) {
		console.error('Error loading project:', error)
	}
}

function handleClickOutside(e: MouseEvent) {
	if (fileMenuRef.value && !fileMenuRef.value.contains(e.target as Node)) {
		isFileMenuOpen.value = false
	}
}

function arrangeImages() {
	if (images.value.length === 0) return

	const rows: { y: number; remainingWidth: number }[] = [{ y: 0, remainingWidth: surfaceWidth.value }]
	
	// Sort images by height (tallest first) to optimize space usage
	const sortedImages = [...images.value].sort((a, b) => b.height - a.height)
	
	for (const image of sortedImages) {
		let placed = false
		
		// Try to fit in existing rows
		for (const row of rows) {
			// Check if image fits with gap
			if (row.remainingWidth >= image.width + arrangeGap.value) {
				// Place image in this row with gap
				const x = surfaceWidth.value - row.remainingWidth
				image.position = { x, y: row.y }
				row.remainingWidth -= (image.width + arrangeGap.value)
				placed = true
				break
			}
		}
		
		// If image doesn't fit in any existing row, create new row
		if (!placed) {
			// Find Y position for new row (below all existing content)
			const maxY = Math.max(...rows.map(row => {
				const imagesInRow = sortedImages.filter(img => img.position.y === row.y)
				return row.y + (imagesInRow.length > 0 ? Math.max(...imagesInRow.map(img => img.height)) : 0)
			}))
			
			// Add gap between rows
			const newY = maxY + (rows.length > 1 ? arrangeGap.value : 0)
			
			// Create new row if it fits within surface height
			if (newY + image.height <= surfaceHeight.value) {
				rows.push({ y: newY, remainingWidth: surfaceWidth.value })
				image.position = { x: 0, y: newY }
				rows[rows.length - 1].remainingWidth -= (image.width + arrangeGap.value)
			}
		}
	}
}

onMounted(() => {
	window.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
	// Cleanup image URLs
	images.value.forEach((img) => URL.revokeObjectURL(img.url))

	// Add these lines to the existing onUnmounted
	window.removeEventListener('keydown', handleKeyDown)
	window.removeEventListener('keyup', handleKeyUp)
	window.removeEventListener('click', handleClickOutside)
})
</script>

<template>
	<div class="px-4 pb-4 h-screen flex flex-col">
		<!-- Existing toolbar -->
		<div class="toolbar mb-4 bg-[#2b2b2b] flex flex-col">
			<!-- Top menu bar -->
			<div class="flex items-center px-1 py-1 bg-[#1e1e1e] text-[#8b8b8b] text-sm">
				<div class="relative" ref="fileMenuRef">
					<button
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded"
						@click.stop="isFileMenuOpen = !isFileMenuOpen"
					>
						File
					</button>
					<div
						v-if="isFileMenuOpen"
						class="absolute top-[40px] left-5 bg-[#2b2b2b] rounded shadow-lg py-1 min-w-[200px] z-10 text-[#8b8b8b]"
					>
						<button
							class="w-full px-4 py-2 text-left hover:bg-[#3a3a3a] flex items-center justify-between"
							@click="newSurface"
						>
							<span>New Surface</span>
							<span class="text-xs text-[#5a5a5a]">Ctrl+N</span>
						</button>
						<button
							class="w-full px-4 py-2 text-left hover:bg-[#3a3a3a] flex items-center justify-between"
							@click="addImage"
						>
							<span>Place Image</span>
							<span class="text-xs text-[#5a5a5a]">Ctrl+P</span>
						</button>
						<div class="border-t border-[#3a3a3a] my-1"></div>
						<button
							class="w-full px-4 py-2 text-left hover:bg-[#3a3a3a] flex items-center justify-between"
							@click="saveProject"
						>
							<span>Save</span>
							<span class="text-xs text-[#5a5a5a]">Ctrl+S</span>
						</button>
						<button
							class="w-full px-4 py-2 text-left hover:bg-[#3a3a3a] flex items-center justify-between"
							@click="loadProject"
						>
							<span>Open</span>
							<span class="text-xs text-[#5a5a5a]">Ctrl+O</span>
						</button>
						<div class="border-t border-[#3a3a3a] my-1"></div>
						<button
							v-if="images.length > 0"
							class="w-full px-4 py-2 text-left hover:bg-[#3a3a3a]"
							@click="downloadPDF"
						>
							Export as PDF
						</button>
					</div>
				</div>
			</div>

			<!-- Options bar -->
			<div class="flex items-center justify-between px-4 py-2 bg-[#2b2b2b] text-[#8b8b8b]">
				<div class="flex items-center gap-4">
					<div class="flex items-center gap-2">
						<span>Surface:</span>
						<input
							v-model.number="surfaceWidth"
							type="number"
							class="w-16 px-2 py-1 bg-[#3a3a3a] rounded border border-[#2b2b2b] focus:border-[#0a84ff]"
							min="10"
							max="1000"
							@input="updateSurfaceDimensions(surfaceWidth, surfaceHeight)"
						/>
						<span>×</span>
						<input
							v-model.number="surfaceHeight"
							type="number"
							class="w-16 px-2 py-1 bg-[#3a3a3a] rounded border border-[#2b2b2b] focus:border-[#0a84ff]"
							min="10"
							max="1000"
							@input="updateSurfaceDimensions(surfaceWidth, surfaceHeight)"
						/>
						<span>cm</span>
					</div>
					<div class="flex items-center gap-2">
						<span>Zoom:</span>
						<span>{{ Math.round(zoom * 100) }}%</span>
					</div>
				</div>
			</div>

			<!-- Tools bar -->
			<div class="flex items-center gap-2 p-2 bg-[#2b2b2b] border-t border-[#232323]">
				<div class="flex items-center gap-1 pr-4 border-r border-[#232323]">
					<button
						v-if="selectedImageIndex !== -1"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Delete (Del)"
						@click="removeImage(selectedImageIndex)"
					>
						Delete
					</button>
					<button
						v-if="selectedImageIndex !== -1"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Duplicate (D)"
						@click="duplicateImage(selectedImageIndex)"
					>
						Duplicate
					</button>
				</div>

				<div class="flex items-center gap-1 px-4 border-r border-[#232323]">
					<button
						v-if="selectedImageIndex !== -1"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Rotate Left (L)"
						@click="rotateLeft(selectedImageIndex)"
					>
						Rotate Left
					</button>
					<button
						v-if="selectedImageIndex !== -1"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Rotate Right (R)"
						@click="rotateRight(selectedImageIndex)"
					>
						Rotate Right
					</button>
				</div>

				<div class="flex items-center gap-1 px-4">
					<button
						v-if="selectedImageIndex !== -1"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Bring Forward (F)"
						@click="bringToFront(selectedImageIndex)"
					>
						Bring Forward
					</button>
					<button
						v-if="selectedImageIndex !== -1"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Send Backward (B)"
						@click="sendToBack(selectedImageIndex)"
					>
						Send Backward
					</button>
				</div>

				<div class="flex items-center gap-1 px-4 border-r border-[#232323]">
					<button
						v-if="images.length > 0"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Arrange Images (A)"
						@click="arrangeImages"
					>
						Arrange
					</button>
					<div class="flex items-center gap-2 ml-2">
						<span class="text-sm">Gap:</span>
						<input
							v-model.number="arrangeGap"
							type="number"
							class="w-16 px-2 py-1 bg-[#3a3a3a] rounded border border-[#2b2b2b] focus:border-[#0a84ff] text-sm"
							min="0"
							max="10"
							step="0.5"
						/>
						<span class="text-sm">cm</span>
					</div>
				</div>
			</div>
		</div>

		<div class="flex gap-4 flex-1">
			<div class="flex-1 overflow-hidden bg-gray-900 rounded-lg">
				<div
					ref="viewerRef"
					class="viewer-container relative w-full h-full flex justify-center"
					@wheel="handleWheel"
					@mousedown="handleViewerMouseDown"
					@mousemove.stop="handleViewerMouseMove"
					@mouseup.stop="handleViewerMouseUp"
					:class="{ 'cursor-grab': isSpacePressed, 'cursor-grabbing': isPanning }"
				>
					<div
						class="viewer absolute"
						:style="{
							width: cmToPixels(surfaceWidth) + 'px',
							height: cmToPixels(surfaceHeight) + 'px',
							transform: `translateX(-50%) scale(${zoom})`,
							left: '50%',
							top: 0,
							transformOrigin: 'top center',
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
									scale: zoom,
									transformOrigin: '0 0',
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
			</div>

			<div v-if="selectedImageIndex !== -1" class="w-64 bg-gray-800 rounded-lg p-4 text-white">
				<h3 class="text-lg font-semibold mb-4">Image Details</h3>
				<div class="space-y-2">
					<div>
						<label class="block text-sm text-gray-400">Filename</label>
						<div class="break-all">{{ images[selectedImageIndex].filename }}</div>
					</div>
					<div>
						<label class="block text-sm text-gray-400">Position</label>
						<div>
							X: {{ Math.round(images[selectedImageIndex].position.x * 10) / 10 }}cm
							Y: {{ Math.round(images[selectedImageIndex].position.y * 10) / 10 }}cm
						</div>
					</div>
					<div>
						<label class="block text-sm text-gray-400">Dimensions</label>
						<div>
							Width: {{ Math.round(images[selectedImageIndex].width * 10) / 10 }}cm
							Height: {{ Math.round(images[selectedImageIndex].height * 10) / 10 }}cm
						</div>
					</div>
					<div>
						<label class="block text-sm text-gray-400">Rotation</label>
						<div>{{ images[selectedImageIndex].rotation }}°</div>
					</div>
					<div>
						<label class="block text-sm text-gray-400">Type</label>
						<div>{{ images[selectedImageIndex].type.toUpperCase() }}</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</template>

<style scoped>
.viewer-container {
	overflow: auto;
	position: relative;
	height: calc(100vh - 120px);
	scrollbar-width: none;
	-ms-overflow-style: none;
	padding: 0 20vw;  /* Only add horizontal padding */
}

.viewer {
	background-image: linear-gradient(45deg, #2c2c2c 25%, transparent 25%),
		linear-gradient(-45deg, #2c2c2c 25%, transparent 25%), 
		linear-gradient(45deg, transparent 75%, #2c2c2c 75%),
		linear-gradient(-45deg, transparent 75%, #2c2c2c 75%);
	background-size: 20px 20px;
	background-position:
		0 0,
		0 10px,
		10px -10px,
		-10px 0px;
	cursor: default;
	user-select: none;
	will-change: transform;
	transform-origin: center center;
}

.viewer-container::-webkit-scrollbar {
	display: none;
}
</style>
