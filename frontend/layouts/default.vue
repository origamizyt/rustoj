<template>
  <main class="r-container">
    <NLayoutHeader>
      <div class="flex flex-row gap-3">
        <img src="/rust-96.png" alt="Rust Logo" class="h-10 cursor-pointer" @click="navigateTo('/')" />
        <div class="flex flex-col self-stretch justify-center">
          <p class="text-xl font-black">RustOJ</p>
        </div>
        <div class="grow"></div>
        <div class="flex flex-col self-stretch justify-center">
          <NButton quaternary type="info" v-if="showContest">
            当前比赛：{{ contestId == 0 ? "默认" : contestId }}
          </NButton>
        </div>
        <div class="flex flex-col self-stretch justify-center">
          <NButton quaternary v-if="showLogin" @click="!token && navigateTo('/login')">
            {{ token ? `欢迎，${token.subject.name}` : "登入" }}
          </NButton>
        </div>
      </div>
    </NLayoutHeader>
    <NLayout>
      <div class="body">
        <slot/>
      </div>
    </NLayout>
    <NLayoutFooter>
      <div class="center">
        Copyright &copy; 2024 origamizyt
      </div>
    </NLayoutFooter>
  </main>
</template>

<script setup lang="ts">
import { NLayout, NLayoutFooter, NLayoutHeader, NButton } from 'naive-ui';

const showContest = ref(false);
const showLogin = ref(false);
const contestId = ref(0);
const path = useRoute().path;
const token = ref<Token>();

if (path == "/") {
  showContest.value = true;
}
else if (path.startsWith("/contest/")) {
  showContest.value = true;
  contestId.value = parseInt(path.split("/")[2]);
}
else {
  showContest.value = false;
}
showLogin.value = path != "/login";

onMounted(() => {
  token.value = getToken();
})

onBeforeRouteUpdate(to => {
  const path = to.path;
  if (path == "/") {
    showContest.value = true;
  }
  else if (path.startsWith("/contest/")) {
    showContest.value = true;
    contestId.value = parseInt(path.split("/")[2]);
  }
  else {
    showContest.value = false;
  }
  showLogin.value = path != "/login";
  token.value = getToken();
})
</script>

<style scoped>
.r-container {
  height: 100vh;
}
.body {
  height: calc(100vh - 137px);
  border-top: 1px solid #ffffff17;
  border-bottom: 1.5px solid #ffffff17;
}

.n-layout-header {
  padding: 14px 24px;
}

.n-layout-footer {
  padding: 24px;
}
</style>