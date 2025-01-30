<script setup lang="ts">
import { ref, onUnmounted, onMounted } from 'vue'
import { PDFDocument } from 'pdf-lib'
import { open } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import ExifReader from 'exif-reader'
import { platform } from '@tauri-apps/plugin-os'

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

interface ProjectData {
	id: string
	name: string
	surfaceWidth: number
	surfaceHeight: number
	images: ImageData[]
	position: { x: number; y: number }
	zoom: number
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
const panStart = ref({ x: 0, y: 0, scrollLeft: 0, scrollTop: 0 })
const isFileMenuOpen = ref(false)
const fileMenuRef = ref<HTMLElement | null>(null)
const arrangeGap = ref(1) // Default 1cm gap
const position = ref({ x: 0, y: 0 })

// Add these refs
const snapThreshold = ref(0.2) // 0.2cm snap threshold
const isSnapping = ref(false)
const snapGuides = ref({ vertical: null as number | null, horizontal: null as number | null })

// Add these refs
const selectionBox = ref({ x: 0, y: 0, width: 0, height: 0 })
const isSelecting = ref(false)
const selectionStart = ref({ x: 0, y: 0 })
const selectedImageIndices = ref<number[]>([])

// Add these refs
const totalMemory = ref(0)
const usedMemory = ref(0)
const systemPlatform = ref('')

// Add these refs for project management
const projects = ref<ProjectData[]>([])
const activeProjectId = ref<string>('')

// Add these refs
const editingProjectId = ref<string>('')
const editingProjectName = ref('')

// Add custom directive for auto-focus
const vFocus = {
	mounted: (el: HTMLElement) => el.focus()
}

function updateSurfaceDimensions(width: number, height: number) {
	surfaceWidth.value = Math.max(10, Math.min(1000, width))
	surfaceHeight.value = Math.max(10, Math.min(1000, height))
	centerSurface()
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
		const newX = pixelsToCm((e.clientX - startPos.value.x) / zoom.value)
		const newY = pixelsToCm((e.clientY - startPos.value.y) / zoom.value)

		// Reset snap guides
		snapGuides.value = { vertical: null, horizontal: null }
		isSnapping.value = false

		let hasSnapped = false

		// Calculate the offset from the original position
		const offsetX = newX - image.position.x
		const offsetY = newY - image.position.y
		let finalOffsetX = offsetX
		let finalOffsetY = offsetY

		// Check snapping for the dragged image
		const moveSpeed = Math.sqrt(offsetX * offsetX + offsetY * offsetY)
		if (moveSpeed < 0.5) { // Only snap when moving very slowly
			images.value.forEach((otherImage, index) => {
				if (!selectedImageIndices.value.includes(index)) {
					// Vertical alignments (X-axis)
					const currentLeft = newX
					const currentRight = newX + image.width
					const currentCenterX = newX + image.width/2
					
					const otherLeft = otherImage.position.x
					const otherRight = otherImage.position.x + otherImage.width
					const otherCenterX = otherImage.position.x + otherImage.width/2

					const verticalAlignments = [
						{ pos: currentLeft, target: otherLeft }, // Left to Left
						{ pos: currentLeft, target: otherRight }, // Left to Right
						{ pos: currentLeft, target: otherCenterX }, // Left to Center
						{ pos: currentRight, target: otherLeft }, // Right to Left
						{ pos: currentRight, target: otherRight }, // Right to Right
						{ pos: currentRight, target: otherCenterX }, // Right to Center
						{ pos: currentCenterX, target: otherLeft }, // Center to Left
						{ pos: currentCenterX, target: otherRight }, // Center to Right
						{ pos: currentCenterX, target: otherCenterX } // Center to Center
					]

					for (const align of verticalAlignments) {
						const distance = Math.abs(align.pos - align.target)
						if (distance < snapThreshold.value) {
							hasSnapped = true
							snapGuides.value.vertical = align.target
							const offset = align.pos - currentLeft
							const snapStrength = 1 - (distance / snapThreshold.value)
							const targetOffset = (align.target - offset) - image.position.x
							finalOffsetX = offsetX + (targetOffset - offsetX) * snapStrength
							break
						}
					}

					// Horizontal alignments (Y-axis)
					const currentTop = newY
					const currentBottom = newY + image.height
					const currentCenterY = newY + image.height/2
					
					const otherTop = otherImage.position.y
					const otherBottom = otherImage.position.y + otherImage.height
					const otherCenterY = otherImage.position.y + otherImage.height/2

					const horizontalAlignments = [
						{ pos: currentTop, target: otherTop }, // Top to Top
						{ pos: currentTop, target: otherBottom }, // Top to Bottom
						{ pos: currentTop, target: otherCenterY }, // Top to Center
						{ pos: currentBottom, target: otherTop }, // Bottom to Top
						{ pos: currentBottom, target: otherBottom }, // Bottom to Bottom
						{ pos: currentBottom, target: otherCenterY }, // Bottom to Center
						{ pos: currentCenterY, target: otherTop }, // Center to Top
						{ pos: currentCenterY, target: otherBottom }, // Center to Bottom
						{ pos: currentCenterY, target: otherCenterY } // Center to Center
					]

					for (const align of horizontalAlignments) {
						const distance = Math.abs(align.pos - align.target)
						if (distance < snapThreshold.value) {
							hasSnapped = true
							snapGuides.value.horizontal = align.target
							const offset = align.pos - currentTop
							const snapStrength = 1 - (distance / snapThreshold.value)
							const targetOffset = (align.target - offset) - image.position.y
							finalOffsetY = offsetY + (targetOffset - offsetY) * snapStrength
							break
						}
					}
				}
			})
		}

		isSnapping.value = hasSnapped

		// Check if any selected image would go out of bounds
		let canMoveX = true
		let canMoveY = true
		
		selectedImageIndices.value.forEach(idx => {
			const selectedImage = images.value[idx]
			const newPosX = selectedImage.position.x + finalOffsetX
			const newPosY = selectedImage.position.y + finalOffsetY
			
			if (newPosX < 0 || newPosX + selectedImage.width > surfaceWidth.value) {
				canMoveX = false
			}
			
			if (newPosY < 0 || newPosY + selectedImage.height > surfaceHeight.value) {
				canMoveY = false
			}
		})
		
		// Move all selected images
		selectedImageIndices.value.forEach(idx => {
			const selectedImage = images.value[idx]
			const newPosX = canMoveX ? selectedImage.position.x + finalOffsetX : selectedImage.position.x
			const newPosY = canMoveY ? selectedImage.position.y + finalOffsetY : selectedImage.position.y
			
			selectedImage.position = {
				x: newPosX,
				y: newPosY
			}
		})
		
		// Store the new position for the dragged image for next offset calculation
		startPos.value = {
			x: e.clientX - cmToPixels(image.position.x) * zoom.value,
			y: e.clientY - cmToPixels(image.position.y) * zoom.value
		}
	}
}

function handleMouseUp() {
	if (draggedImageIndex.value !== -1) {
		images.value[draggedImageIndex.value].isResizing = false
	}
	isDragging.value = false
	draggedImageIndex.value = -1
	isSnapping.value = false
	snapGuides.value = { vertical: null, horizontal: null }
}

function rotateLeft(index: number) {
	if (selectedImageIndices.value.includes(index)) {
		selectedImageIndices.value.forEach(idx => {
			images.value[idx].rotation = (images.value[idx].rotation - 90) % 360
		})
	} else {
		images.value[index].rotation = (images.value[index].rotation - 90) % 360
	}
}

function rotateRight(index: number) {
	images.value[index].rotation = (images.value[index].rotation + 90) % 360
}

function handleMouseDown(e: MouseEvent, index: number) {
	// Only allow left click for image dragging
	if (e.button !== 0 || isSpacePressed.value) return
	
	e.stopPropagation() // Stop event from bubbling to viewer
	
	isDragging.value = true
	draggedImageIndex.value = index

	if (e.shiftKey) {
		// Shift+click - toggle this image in selection
		const selectionIndex = selectedImageIndices.value.indexOf(index)
		if (selectionIndex === -1) {
			selectedImageIndices.value.push(index)
			selectedImageIndex.value = index
		} else {
			selectedImageIndices.value.splice(selectionIndex, 1)
			selectedImageIndex.value = selectedImageIndices.value.length > 0 ? 
				selectedImageIndices.value[selectedImageIndices.value.length - 1] : -1
		}
	} else if (selectedImageIndices.value.includes(index)) {
		// Clicking an already selected image - keep the selection
		selectedImageIndex.value = index
	} else {
		// Clicking a new image - select only this one
		selectedImageIndices.value = [index]
		selectedImageIndex.value = index
	}
	
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
		
		const rect = viewerRef.value?.getBoundingClientRect()
		if (!rect) return

		// Calculate new zoom
		const delta = -Math.sign(e.deltaY) * 0.1
		const newZoom = Math.max(minZoom, Math.min(maxZoom, zoom.value + delta))
		
		if (newZoom !== zoom.value) {
			// Get the center of the container (horizontally only)
			const containerCenterX = rect.width / 2

			// Get the surface center relative to container before zoom (horizontally only)
			const surfaceCenterX = (containerCenterX - position.value.x) / zoom.value

			// Update zoom
			zoom.value = newZoom

			// Update position to keep surface centered horizontally only
			position.value = {
				x: containerCenterX - surfaceCenterX * newZoom,
				y: position.value.y // Keep vertical position unchanged
			}
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
			case 'KeyT':
				if (images.length > 0) {
					arrangeAsTable()
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
	if (e.button === 1 || (e.button === 0 && isSpacePressed.value)) {
		e.preventDefault()
		isPanning.value = true
		panStart.value = {
			x: e.clientX - position.value.x,
			y: e.clientY - position.value.y
		}
	} else if (e.button === 0) {
		const target = e.target as HTMLElement
		
		if (target.closest('.viewer') && !target.closest('.image-container')) {
			isSelecting.value = true
			const rect = viewerRef.value?.getBoundingClientRect()
			if (!rect) return
			
			const surfaceX = e.clientX - rect.left - position.value.x - 32
			const surfaceY = e.clientY - rect.top - position.value.y - 32
			
			selectionStart.value = {
				x: surfaceX,
				y: surfaceY
			}
			selectionBox.value = {
				x: surfaceX,
				y: surfaceY,
				width: 0,
				height: 0
			}
			
			if (!e.shiftKey) {
				selectedImageIndices.value = []
				selectedImageIndex.value = -1
			}
		}
	}
}

function handleViewerMouseMove(e: MouseEvent) {
	if (isPanning.value) {
		position.value = {
			x: e.clientX - panStart.value.x,
			y: e.clientY - panStart.value.y
		}
	} else if (isSelecting.value) {
		const rect = viewerRef.value?.getBoundingClientRect()
		if (!rect) return
		
		const surfaceX = e.clientX - rect.left - position.value.x - 32
		const surfaceY = e.clientY - rect.top - position.value.y - 32
		
		selectionBox.value = {
			x: Math.min(selectionStart.value.x, surfaceX),
			y: Math.min(selectionStart.value.y, surfaceY),
			width: Math.abs(surfaceX - selectionStart.value.x),
			height: Math.abs(surfaceY - selectionStart.value.y)
		}
		
		// Check which images are in the selection box
		images.value.forEach((image, index) => {
			const imageRect = {
				x: cmToPixels(image.position.x),
				y: cmToPixels(image.position.y),
				width: cmToPixels(image.width),
				height: cmToPixels(image.height)
			}
			
			if (isRectIntersecting(selectionBox.value, imageRect)) {
				if (!selectedImageIndices.value.includes(index)) {
					selectedImageIndices.value.push(index)
					selectedImageIndex.value = index
				}
			}
		})
	}
}

function handleViewerMouseUp() {
	isPanning.value = false
	isSelecting.value = false
}

function removeImage(index: number) {
	// Remove all selected images if the clicked image is selected
	if (selectedImageIndices.value.includes(index)) {
		selectedImageIndices.value.forEach(idx => {
			URL.revokeObjectURL(images.value[idx].url)
		})
		images.value = images.value.filter((_, idx) => !selectedImageIndices.value.includes(idx))
		selectedImageIndices.value = []
		selectedImageIndex.value = -1
	} else {
		URL.revokeObjectURL(images.value[index].url)
		images.value.splice(index, 1)
		selectedImageIndex.value = -1
	}
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
	centerSurface()
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
		const currentProject = projects.value.find(p => p.id === activeProjectId.value)
		if (!currentProject) return
		
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
		link.download = `${currentProject.name}.json`
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
			filters: [{ name: 'JSON', extensions: ['json'] }],
		})

		if (selected) {
			const fileContent = await readFile(selected as string)
			const projectData = JSON.parse(new TextDecoder().decode(fileContent))
			
			// Convert array data back to Uint8Array for images
			const processedImages = projectData.images.map((img: any) => ({
				...img,
				data: new Uint8Array(img.data)
			}))
			
			// Create new project from loaded data
			const newProject: ProjectData = {
				id: crypto.randomUUID(),
				name: (selected as string).split(/[/\\]/).pop()?.replace('.json', '') || 'Untitled',
				surfaceWidth: projectData.surfaceWidth,
				surfaceHeight: projectData.surfaceHeight,
				images: processedImages,
				position: projectData.position || { x: 0, y: 0 },
				zoom: projectData.zoom || 1
			}
			projects.value.push(newProject)
			switchProject(newProject.id)
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

// Add new table function
function arrangeAsTable() {
	if (images.value.length === 0) return

	// Calculate optimal grid dimensions
	const count = images.value.length
	const aspectRatio = surfaceWidth.value / surfaceHeight.value
	const cols = Math.ceil(Math.sqrt(count * aspectRatio))
	const rows = Math.ceil(count / cols)

	// Calculate cell dimensions with gaps
	const totalGapWidthSpace = arrangeGap.value * (cols - 1)
	const totalGapHeightSpace = arrangeGap.value * (rows - 1)
	const cellWidth = (surfaceWidth.value - totalGapWidthSpace) / cols
	const cellHeight = (surfaceHeight.value - totalGapHeightSpace) / rows

	// Arrange images in grid
	images.value.forEach((image, index) => {
		const row = Math.floor(index / cols)
		const col = index % cols

		// Set position to cell position with gaps
		image.position = {
			x: col * (cellWidth + arrangeGap.value),
			y: row * (cellHeight + arrangeGap.value)
		}

		// Set dimensions to match cell size
		image.width = cellWidth
		image.height = cellHeight
	})
}

// Keep the original arrange function
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

// Update centerSurface function
function centerSurface() {
	const container = viewerRef.value
	if (!container) return

	const rect = container.getBoundingClientRect()
	position.value = {
		x: (rect.width - cmToPixels(surfaceWidth.value) * zoom.value) / 2,
		y: 20 // Fixed top padding
	}
}

function constrainPosition(index: number) {
	const image = images.value[index]
	if (!image) return

	// Constrain X position
	image.position.x = Math.max(0, Math.min(surfaceWidth.value - image.width, image.position.x))
	
	// Constrain Y position
	image.position.y = Math.max(0, Math.min(surfaceHeight.value - image.height, image.position.y))
}

// Update isRectIntersecting to be more precise
function isRectIntersecting(r1: { x: number, y: number, width: number, height: number }, 
						  r2: { x: number, y: number, width: number, height: number }) {
	const r1Right = r1.x + r1.width
	const r1Bottom = r1.y + r1.height
	const r2Right = r2.x + r2.width
	const r2Bottom = r2.y + r2.height
	
	return !(r2.x > r1Right ||
			r2Right < r1.x ||
			r2.y > r1Bottom ||
			r2Bottom < r1.y)
}

// Function to update memory info
async function updateMemoryInfo() {
	try {
		// For now, let's use navigator.deviceMemory as a fallback
		if (navigator.deviceMemory) {
			totalMemory.value = navigator.deviceMemory
			usedMemory.value = Math.round(navigator.deviceMemory * 0.7) // Rough estimate
		}
		systemPlatform.value = await platform()
	} catch (error) {
		console.error('Error getting system info:', error)
	}
}

// Function to create a new project
function createNewProject(name: string = 'Untitled') {
	// Save current project if exists
	if (activeProjectId.value) {
		const currentProject = projects.value.find(p => p.id === activeProjectId.value)
		if (currentProject) {
			currentProject.surfaceWidth = surfaceWidth.value
			currentProject.surfaceHeight = surfaceHeight.value
			currentProject.images = images.value.map(img => ({
				...img,
				data: img.data,
				url: ''
			}))
			currentProject.position = { ...position.value }
			currentProject.zoom = zoom.value
		}
	}

	const newProject: ProjectData = {
		id: crypto.randomUUID(),
		name,
		surfaceWidth: 100,
		surfaceHeight: 100,
		images: [],
		position: { x: 0, y: 0 },
		zoom: 1
	}
	projects.value.push(newProject)
	activeProjectId.value = newProject.id
	
	// Clear current state
	images.value = []
	surfaceWidth.value = 100
	surfaceHeight.value = 100
	position.value = { x: 0, y: 0 }
	zoom.value = 1
	centerSurface()
}

// Function to switch between projects
function switchProject(projectId: string) {
	// Save current project state
	if (activeProjectId.value) {
		const currentProject = projects.value.find(p => p.id === activeProjectId.value)
		if (currentProject) {
			currentProject.surfaceWidth = surfaceWidth.value
			currentProject.surfaceHeight = surfaceHeight.value
			currentProject.images = images.value.map(img => ({
				...img,
				data: img.data,
				url: '' // Don't store URLs as they need to be recreated
			}))
			currentProject.position = { ...position.value }
			currentProject.zoom = zoom.value
		}
	}
	
	// Load new project state
	const project = projects.value.find(p => p.id === projectId)
	if (project) {
		activeProjectId.value = projectId
		surfaceWidth.value = project.surfaceWidth
		surfaceHeight.value = project.surfaceHeight
		images.value = project.images.map(img => {
			const blob = new Blob([img.data], { type: `image/${img.type}` })
			const url = URL.createObjectURL(blob)
			return {
				...img,
				url
			}
		})
		position.value = { ...project.position }
		zoom.value = project.zoom
		centerSurface()
	}
}

// Function to close project
function closeProject(projectId: string) {
	const index = projects.value.findIndex(p => p.id === projectId)
	if (index !== -1) {
		projects.value.splice(index, 1)
		if (projectId === activeProjectId.value) {
			activeProjectId.value = projects.value[Math.max(0, index - 1)]?.id || ''
			if (activeProjectId.value) {
				switchProject(activeProjectId.value)
			}
		}
	}
}

// Add function to handle tab rename
function startRenameProject(project: ProjectData, event: MouseEvent) {
	event.stopPropagation()
	editingProjectId.value = project.id
	editingProjectName.value = project.name
}

function finishRenameProject() {
	if (editingProjectId.value) {
		const project = projects.value.find(p => p.id === editingProjectId.value)
		if (project && editingProjectName.value.trim()) {
			project.name = editingProjectName.value.trim()
		}
		editingProjectId.value = ''
		editingProjectName.value = ''
	}
}

onMounted(() => {
	window.addEventListener('click', handleClickOutside)
	// Create initial project if none exists
	if (projects.value.length === 0) {
		createNewProject()
	}
	centerSurface()
	updateMemoryInfo()
	const memoryInterval = setInterval(updateMemoryInfo, 2000)
	
	onUnmounted(() => {
		clearInterval(memoryInterval)
	})
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

			<!-- Project tabs -->
			<div class="flex items-center gap-1 px-2 py-1 bg-[#1e1e1e] border-t border-[#3a3a3a] overflow-x-auto">
				<div
					v-for="project in projects"
					:key="project.id"
					class="flex items-center gap-2 px-3 py-1 rounded-t cursor-pointer text-sm"
					:class="[
						project.id === activeProjectId 
							? 'bg-[#2b2b2b] text-white' 
							: 'bg-[#252525] text-[#8b8b8b] hover:bg-[#2b2b2b]'
					]"
					@click="switchProject(project.id)"
					@dblclick="startRenameProject(project, $event)"
				>
					<div class="min-w-[60px]">
						<input
							v-if="editingProjectId === project.id"
							v-model="editingProjectName"
							class="w-full px-1 py-0.5 bg-[#1e1e1e] text-white rounded border border-[#0a84ff] focus:outline-none"
							@keyup.enter="finishRenameProject"
							@blur="finishRenameProject"
							@click.stop
							ref="editInput"
							v-focus
						/>
						<span v-else>{{ project.name }}</span>
					</div>
					<button
						class="opacity-50 hover:opacity-100"
						@click.stop="closeProject(project.id)"
					>
						×
					</button>
				</div>
				
				<!-- New project button -->
				<button
					class="px-3 py-1 text-[#8b8b8b] hover:bg-[#3a3a3a] rounded text-sm"
					@click="createNewProject()"
				>
					+
				</button>
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
					<button
						v-if="images.length > 0"
						class="px-3 py-1 hover:bg-[#3a3a3a] rounded text-sm"
						title="Table View (T)"
						@click="arrangeAsTable"
					>
						Table
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
				<!-- Horizontal Ruler -->
				<div class="h-8 ml-8 relative bg-[#2b2b2b] border-b border-[#3a3a3a] overflow-hidden">
					<div 
						class="absolute h-full"
						:style="{
							transform: `translate(${position.x}px, 0) scale(${zoom})`,
							transformOrigin: 'left',
							width: cmToPixels(surfaceWidth) + 'px'
						}"
					>
						<div 
							v-for="i in Math.ceil(surfaceWidth)"
							:key="i"
							class="absolute top-0 h-full flex items-end"
							:style="{ left: `${cmToPixels(i - 1)}px` }"
						>
							<div class="h-2 border-l border-[#8b8b8b]"></div>
							<div 
								v-if="(i - 1) % 5 === 0" 
								class="absolute bottom-0 left-0 text-[10px] text-[#8b8b8b] transform -translate-x-1/2"
							>
								{{ i - 1 }}
							</div>
						</div>
					</div>
				</div>

				<div class="flex">
					<!-- Vertical Ruler -->
					<div class="w-8 relative bg-[#2b2b2b] border-r border-[#3a3a3a] overflow-hidden">
						<div 
							class="absolute w-full"
							:style="{
								transform: `translate(0, ${position.y}px) scale(${zoom})`,
								transformOrigin: 'top',
								height: cmToPixels(surfaceHeight) + 'px'
							}"
						>
							<div 
								v-for="i in Math.ceil(surfaceHeight)"
								:key="i"
								class="absolute left-0 w-full flex items-center"
								:style="{ top: `${cmToPixels(i - 1)}px` }"
							>
								<div class="w-2 border-t border-[#8b8b8b]"></div>
								<div 
									v-if="(i - 1) % 5 === 0" 
									class="absolute left-3 text-[10px] text-[#8b8b8b] transform -translate-y-1/2"
								>
									{{ i - 1 }}
								</div>
							</div>
						</div>
					</div>

					<div
						ref="viewerRef"
						class="viewer-container flex-1 relative"
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
								transform: `translate(${position.x}px, ${position.y}px) scale(${zoom})`,
								transformOrigin: 'center center',
							}"
							@mousemove="handleMouseMove"
							@mouseup="handleMouseUp"
							@mouseleave="handleMouseUp"
						>
							<template v-if="images.length > 0">
								<!-- Add guide lines -->
								<div v-if="isSnapping && draggedImageIndex !== -1">
									<!-- Vertical guide -->
									<div
										v-if="snapGuides.vertical"
										class="absolute border-l border-blue-500 border-dashed h-full"
										:style="{
											left: `${cmToPixels(snapGuides.vertical)}px`,
											top: '0'
										}"
									></div>
									<!-- Horizontal guide -->
									<div
										v-if="snapGuides.horizontal"
										class="absolute border-t border-blue-500 border-dashed w-full"
										:style="{
											top: `${cmToPixels(snapGuides.horizontal)}px`,
											left: '0'
										}"
									></div>
								</div>

								<!-- Images -->
								<div
									v-for="(image, index) in images"
									:key="index"
									class="absolute image-container"
									:class="{
										'outline outline-2 outline-blue-500': selectedImageIndices.includes(index),
										'outline outline-2 outline-blue-700': isSnapping && index === draggedImageIndex
									}"
									:style="{
										transform: `translate(${cmToPixels(image.position.x)}px, ${cmToPixels(image.position.y)}px) rotate(${image.rotation}deg)`,
										width: `${cmToPixels(image.width)}px`,
										height: `${cmToPixels(image.height)}px`,
										cursor: 'pointer',
										transformOrigin: '0 0',
										zIndex: '1'
									}"
									@mousedown.stop="(e) => handleMouseDown(e, index)"
								>
									<div 
										class="relative group h-full w-full"
										:class="{ 'cursor-grabbing': isDragging && draggedImageIndex === index, 'cursor-grab': !isDragging }"
									>
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

								<!-- Selection box (moved to top) -->
								<div
									v-if="isSelecting"
									class="absolute border-2 border-blue-500 bg-transparent pointer-events-none"
									:style="{
										left: `${selectionBox.x}px`,
										top: `${selectionBox.y}px`,
										width: `${selectionBox.width}px`,
										height: `${selectionBox.height}px`,
										zIndex: '2'
									}"
								></div>
							</template>
							<div v-else class="h-full flex items-center justify-center text-gray-400">Add images to create a PDF</div>
						</div>
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
						<div class="flex gap-2 items-center">
							<div class="flex items-center gap-1">
								<span class="text-sm">X:</span>
								<input
									v-model.number="images[selectedImageIndex].position.x"
									type="number"
									class="w-20 px-2 py-1 bg-[#3a3a3a] rounded border border-[#2b2b2b] focus:border-[#0a84ff] text-sm"
									step="0.1"
									@input="constrainPosition(selectedImageIndex)"
								/>
							</div>
							<div class="flex items-center gap-1">
								<span class="text-sm">Y:</span>
								<input
									v-model.number="images[selectedImageIndex].position.y"
									type="number"
									class="w-20 px-2 py-1 bg-[#3a3a3a] rounded border border-[#2b2b2b] focus:border-[#0a84ff] text-sm"
									step="0.1"
									@input="constrainPosition(selectedImageIndex)"
								/>
							</div>
							<span class="text-sm text-gray-400">cm</span>
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

		<div class="flex items-center justify-between px-4 py-2 bg-[#2b2b2b] text-[#8b8b8b] border-t border-[#3a3a3a]">
			<div class="flex items-center justify-between w-full">
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
				<div class="flex items-center gap-2 text-sm">
					<span class="capitalize">{{ systemPlatform }}:</span>
					<span>{{ usedMemory }}GB / {{ totalMemory }}GB</span>
				</div>
			</div>
		</div>
	</div>
</template>

<style scoped>
.viewer-container {
	height: calc(100vh - 190px); /* Account for ruler height and bottom bar */
	overflow: hidden;
}

.viewer {
	position: absolute;
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
}

/* Add ruler styles */
.ruler-mark {
	position: absolute;
	background: #8b8b8b;
}
</style>
