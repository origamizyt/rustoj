<template>
  <div class="flex flex-col h-full justify-center">
    <div class="flex flex-row justify-center">
      <div class="w-[350px] text-center">
        <img src="/rust-96.png" alt="Rust Logo" class="inline-block mb-3 h-[90px]"/>
        <p class="text-3xl font-black mb-1">登入 RustOJ</p>
        <p class="text-gray-500 text-xs mb-4">用户管理将在续行版本加入</p>
        <NInputGroup class="text-left">
          <NInput placeholder="输入用户名..." size="large" v-model:value="name"/>
          <NButton size="large" type="primary" @click="login" :disabled="!name.length">
            登入
            <template #icon>
              <IconLogin color="#000" size="20"/>
            </template>
          </NButton>
        </NInputGroup>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { IconLogin } from '@tabler/icons-vue';
import { NButton, NInput, NInputGroup, useMessage } from 'naive-ui';
import backend from '~/utils/api';

useHead({
  title: '登入 | RustOJ'
})

const name = ref('');
const message = useMessage();

function login() {
  backend.post<{ token: string }>("/users/login", {
    name: name.value
  }, true)
  .then(({ token }) => {
    if (import.meta.env.DEV) {
      document.cookie = `rustoj-token=${token}`;
    }
  })
  .then(() => {
    navigateTo("/");
  })
  .catch((error: Failure) => {
    message.error(`[${error.reason}] ${error.message}`, {
      duration: 4000,
      showIcon: true
    });
  })
}
</script>