import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import SnakeGame from './components/SnakeGame.vue'

const routes: RouteRecordRaw[] = [
	{
		path: '/',
		name: 'home',
		component: () => import('./layouts/Default.vue'),
		children: [
			{
				path: '',
				component: () => import('./pages/Home.vue'),
			},
		],
	},
	{
		path: '/snake',
		name: 'snake',
		component: SnakeGame,
	},
	{
		path: '/pdf-viewer',
		name: 'pdf-viewer',
		component: () => import('./pages/PdfViewer.vue'),
	},
]

export const router = createRouter({
	history: createWebHistory(),
	routes,
})
