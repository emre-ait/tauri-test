<template>
	<div class="snake-game">
		<canvas ref="gameCanvas" width="400" height="400"></canvas>
		<div class="score">Score: {{ score }}</div>
		<button v-if="!isPlaying" @click="startGame">Start Game</button>
	</div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

interface Position {
  x: number
  y: number
}

const gameCanvas = ref<HTMLCanvasElement | null>(null)
const ctx = ref<CanvasRenderingContext2D | null>(null)
const snake = ref<Position[]>([{ x: 10, y: 10 }])
const food = ref<Position>({ x: 5, y: 5 })
const direction = ref<string>('right')
const score = ref<number>(0)
const isPlaying = ref<boolean>(false)
const gameInterval = ref<ReturnType<typeof setInterval> | null>(null)

const gridSize = 20
const gridCount = 20

const startGame = () => {
  if (isPlaying.value) return

  snake.value = [{ x: 10, y: 10 }]
  score.value = 0
  direction.value = 'right'
  isPlaying.value = true
  generateFood()
  gameInterval.value = setInterval(gameLoop, 100)
}

const generateFood = () => {
  food.value = {
    x: Math.floor(Math.random() * gridCount),
    y: Math.floor(Math.random() * gridCount)
  }
}

const gameLoop = () => {
  const head = { ...snake.value[0] }

  switch (direction.value) {
    case 'up':
      head.y--
      break
    case 'down':
      head.y++
      break
    case 'left':
      head.x--
      break
    case 'right':
      head.x++
      break
  }

  // Check for collisions
  if (
    head.x < 0 ||
    head.x >= gridCount ||
    head.y < 0 ||
    head.y >= gridCount ||
    snake.value.some(segment => segment.x === head.x && segment.y === head.y)
  ) {
    endGame()
    return
  }

  snake.value.unshift(head)

  // Check if snake ate food
  if (head.x === food.value.x && head.y === food.value.y) {
    score.value += 10
    generateFood()
  } else {
    snake.value.pop()
  }

  draw()
}

const draw = () => {
  if (!ctx.value || !gameCanvas.value) return

  // Clear canvas
  ctx.value.fillStyle = '#000000'
  ctx.value.fillRect(0, 0, gameCanvas.value.width, gameCanvas.value.height)

  // Draw snake
  ctx.value.fillStyle = '#00ff00'
  snake.value.forEach(segment => {
    ctx.value!.fillRect(
      segment.x * gridSize,
      segment.y * gridSize,
      gridSize - 1,
      gridSize - 1
    )
  })

  // Draw food
  ctx.value.fillStyle = '#ff0000'
  ctx.value.fillRect(
    food.value.x * gridSize,
    food.value.y * gridSize,
    gridSize - 1,
    gridSize - 1
  )
}

const endGame = () => {
  isPlaying.value = false
  if (gameInterval.value) {
    clearInterval(gameInterval.value)
  }
}

const handleKeyPress = (event: KeyboardEvent) => {
  switch (event.key) {
    case 'ArrowUp':
      if (direction.value !== 'down') direction.value = 'up'
      break
    case 'ArrowDown':
      if (direction.value !== 'up') direction.value = 'down'
      break
    case 'ArrowLeft':
      if (direction.value !== 'right') direction.value = 'left'
      break
    case 'ArrowRight':
      if (direction.value !== 'left') direction.value = 'right'
      break
  }
}

onMounted(() => {
  if (gameCanvas.value) {
    ctx.value = gameCanvas.value.getContext('2d')
    window.addEventListener('keydown', handleKeyPress)
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyPress)
  if (gameInterval.value) {
    clearInterval(gameInterval.value)
  }
})
</script>

<style scoped>
.snake-game {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  padding: 2rem;
}

canvas {
  border: 2px solid #333;
}

.score {
  font-size: 1.5rem;
  font-weight: bold;
}

button {
  padding: 0.5rem 1rem;
  font-size: 1rem;
  cursor: pointer;
  background-color: #4CAF50;
  color: white;
  border: none;
  border-radius: 4px;
}

button:hover {
  background-color: #45a049;
}
</style> 