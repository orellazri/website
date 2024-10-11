import antfu from '@antfu/eslint-config'

export default antfu({
  vue: true,
  typescript: true,
  astro: true,
  formatters: {
    astro: true,
    css: true,
  },
  rules: {
    'style/no-tabs': 'off',
    'style/no-mixed-spaces-and-tabs': 'off',
    'ts/no-empty-object-type': 'off',
    'ts/method-signature-style': 'off',
  },
})
