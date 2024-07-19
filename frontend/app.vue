<script setup lang="ts">
import { darkTheme, NConfigProvider, NMessageProvider } from 'naive-ui';
import '@fontsource/inter';
</script>

<template>
  <NConfigProvider :theme="darkTheme">
    <NMessageProvider>
      <NuxtLayout>
        <NuxtPage/>
      </NuxtLayout>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
:global(html, body) {
  margin: 0;
}

* {
  font-family: 'Inter', sans-serif;
}

.center {
  text-align: center;
}
</style>