<template>
  <NGrid :cols="4" class="h-full">
    <NGi :span="3">
      <NSplit direction="horizontal" class="h-full" :max="0.75" :min="0.25">
        <template #1>
          <div class="h-full p-4">
            <div class="flex flex-row gap-1">
              <p class="text-3xl">
                #{{ problem?.id }}
                {{ problem?.name }}
              </p>
              <p class="text-md text-gray-500 self-end pb-0.5">| {{ problem?.cases }} 个测试点, {{ problem?.score }} 分</p>
            </div>
            <p class="text-md">
              评测方式:
              <span class="uppercase tracking-wider font-bold text-[12px]">
                {{ problem?.type }}
              </span>
            </p>
            <div v-html="problem && marked(problem.desc)" class="markdown"></div>
          </div>
        </template>
        <template #2>
          <div class="h-full pl-1 bg-[#1a1b26]">
            <CodeMirror
              tab
              :lang="rust()"
              :extensions="[tokyoNight, indentUnit.of('    ')]"
              dark
              basic
              v-model="code"/>
          </div>
        </template>
        <template #resize-trigger>
          <div class="flex flex-col justify-center h-full text-center w-5 trigger -translate-x-2">
            <div class="flex flex-row justify-center">
              <IconArrowsHorizontal :size="20" color="#fff"/>
            </div>
          </div>
        </template>
      </NSplit>
    </NGi>
    <NGi :span="1">
      <div class="border-l-2 border-gray-800 h-full p-2">
        <div class="flex flex-row gap-1">
          <NButton type="info" size="small">
            <IconPlayerPlay :size="20" color="#fff"/>
          </NButton>
          <NButton type="error" size="small">
            <IconPlayerStop :size="20" color="#fff"/>
          </NButton>
        </div>
        <div>
          {{ jobs }}
        </div>
      </div>
    </NGi>
  </NGrid>
</template>

<script setup lang="ts">
import { IconArrowsHorizontal, IconPlayerPlay, IconPlayerStop } from '@tabler/icons-vue';
import { NGrid, NGi, NSplit, NButton } from 'naive-ui';
import CodeMirror from 'vue-codemirror6';
import { rust } from '@codemirror/lang-rust';
import { tokyoNight } from '@uiw/codemirror-theme-tokyo-night';
import { indentUnit } from '@codemirror/language';
import backend from '~/utils/api';
import { marked } from 'marked';

const route = useRoute();

const code = ref('// insert code here');
const problem = ref<Problem>();
const jobs = ref<Job[]>([]);

if (!route.query.id) {
  navigateTo("/");
}

onMounted(() => {
  backend.get<Problem>(`/problems/${route.query.id}`, {}, true)
  .then(p => {
    problem.value = p;
  });
  backend.get<Job[]>('/jobs', {
    userId: getToken()!.subject.id,
    problemId: route.query.id
  })
  .then(js => jobs.value = js);
})

useHead({
  title: '问题 | RustOJ'
})
</script>

<style scoped>
.trigger {
  background: linear-gradient(90deg, transparent 40%, #1f2937 40%, #1f2937 60%, transparent 60%);
}

.vue-codemirror * {
  font-family: 'Consolas', monospace;
}

.markdown {
  margin-top: 20px;
}

.markdown :deep(h1) {
  font-size: 20px;
  font-weight: bold;
  margin-bottom: 10px;
  margin-top: 10px;
}

.markdown :deep(pre code) {
  font-family: 'Consolas', monospace;
  display: block;
  padding: 3px 10px;
}

.markdown :deep(pre) {
  border: 1px solid #1e293b;
  border-radius: 5px;
  transition: all .2s ease;
  margin-bottom: 5px;
}

.markdown :deep(pre:has(.language-input)::before) {
  display: block;
  font-size: 10px;
  content: 'input';
  text-transform: uppercase;
  border-bottom: 1px solid #1e293b;
  padding: 2px 10px;
}

.markdown :deep(pre:has(.language-output)::before) {
  display: block;
  font-size: 10px;
  content: 'output';
  text-transform: uppercase;
  border-bottom: 1px solid #1e293b;
  padding: 2px 10px;
}

.markdown :deep(pre:hover) {
  background-color: #0f172a;
}

</style>