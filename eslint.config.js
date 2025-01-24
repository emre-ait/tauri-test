import vue from 'eslint-plugin-vue'
import pluginVue from 'eslint-plugin-vue'
import vueParser from 'vue-eslint-parser'
import typescriptPlugin from '@typescript-eslint/eslint-plugin'

export default [
	...pluginVue.configs['flat/recommended'],
	{
		files: ['**/*.ts', '**/*.vue'],
		languageOptions: {
			parser: vueParser,
			parserOptions: {
				parser: '@typescript-eslint/parser',
				ecmaVersion: 2020,
				sourceType: 'module',
				extraFileExtensions: ['.vue'],
				project: './tsconfig.json',
			},
		},
		plugins: {
			vue,
			'@typescript-eslint': typescriptPlugin,
		},
		rules: {
			'vue/multi-word-component-names': 'off',
			'vue/html-indent': ['error', 'tab'],
			'vue/html-self-closing': 'off',
			'@typescript-eslint/no-unused-vars': 'error',
			'@typescript-eslint/explicit-function-return-type': 'off',
			'vue/multi-word-component-names': 'off',
			'vue/no-unused-vars': 'error',
			'vue/no-use-v-if-with-v-for': 'off',
			'vue/singleline-html-element-content-newline': 'off',
			'@typescript-eslint/no-explicit-any': 'off',
			'vue/max-attributes-per-line': [
				'error',
				{
					singleline: {
						max: 5,
					},
					multiline: {
						max: 1,
					},
				},
			],
		},
	},
]
