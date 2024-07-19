<template>
  <NGrid :cols="4" class="h-full">
    <NGi :span="1">
      <div class="sider p-4">
        <div class="flex flex-row mb-2 gap-1">
          <p class="text-lg">排行榜</p>
          <p class="text-xs text-gray-500 self-end pb-0.5">| Rank List</p>
        </div>
        <NTable bordered size="small">
          <thead>
            <tr>
              <th>排名</th>
              <th>用户名</th>
              <th>分数</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="rank in ranklist">
              <td>#{{ rank.rank }}</td>
              <td>{{ rank.user.name }}</td>
              <td>{{ rank.scores.reduce((a, b) => a+b) }}</td>
            </tr>
          </tbody>
        </NTable>
      </div>
    </NGi>
    <NGi :span="3">
      <div class="p-4">
        <div class="flex flex-row gap-2">
          <p class="text-2xl">问题列表</p>
          <p class="text-sm text-gray-500 self-end pb-0.5">| Problem List</p>
        </div>
        <div class="flex flex-row flex-wrap justify-space-between mt-3">
          <NCard 
            v-for="problem, i in problems" 
            class="max-w-[33%] cursor-pointer" size="small" 
            hoverable :key="problem.id"
            @click="navigateTo(`/problem?id=${problem.id}`)">
            <div class="flex flex-row">
              <div class="flex-grow">
                <div class="flex flex-row gap-1 mb-0.5">
                  <p class="text-xl">{{ problem.name }}</p>
                  <p class="text-xs text-gray-500 self-end pb-0.5">| {{ problem.cases }} 个测试点, {{ problem.score }} 分</p>
                </div>
                <p class="text-xs text-gray-500">
                  评测方式:
                  <span class="uppercase tracking-wider font-bold text-[10px]">
                    {{ problem.type }}
                  </span>
                </p>
              </div>
              <div class="flex flex-col self-stretch justify-center items-center" v-if="jobs[i]">
                <component :is="iconOf(jobs[i].result)" :color="colorOf(jobs[i].result)" :size="20"/>
                <p class="text-xs" :style="{ color: colorOf(jobs[i].result) }">{{ jobs[i].result }}</p>
              </div>
              <div class="flex flex-col self-stretch justify-center items-center" v-else>
                <IconPercentage0 color="#fff" :size="20"/>
                <p class="text-xs">Untested</p>
              </div>
            </div>
          </NCard>
        </div>
      </div>
    </NGi>
  </NGrid>
</template>

<script setup lang="ts">
import { NGrid, NGi, NTable, NCard } from 'naive-ui';
import { IconBug, IconPercentage0, IconTrophy, IconX, type Icon } from '@tabler/icons-vue';
import backend from '~/utils/api';

const ranklist = ref<Ranking[]>([]);
const problems = ref<Problem[]>([]);
const jobs = ref<(Job | undefined)[]>([]);

onMounted(() => {
  backend.get<Ranking[]>("/contests/0/ranklist", {
    scoringRule: 'highest',
    tieBreaker: 'submission_time',
  }, true)
  .then(rl => ranklist.value = rl);
  backend.get<Problem[]>("/contests/0/problems", {}, true)
  .then(ps => problems.value = ps)
  .then(ps => {
    console.log(ps);
    const token = getToken();
    if (token) {
      Promise.all(ps.map(problem => 
        backend.get<Job[]>("/jobs", {
          userId: token.subject.id.toString(),
          problemId: problem.id.toString(),
          state: 'Finished'
        }, true)
        .then(jobs => {
          console.log(jobs)
          return jobs.sort((a, b) => 
            b.score - a.score || 
            new Date(a.createdTime).getTime() - new Date(b.createdTime).getTime()
          )[0];
        })
      ))
      .then(js => {
        jobs.value = js;
      })
    }
  });
})

function colorOf(result: Status): string {
  switch (result) {
    case 'Accepted': return '#63e2b7';
    case 'Wrong Answer': return '#e88080';
    default: return '#f2c97d';
  }
}

function iconOf(result: Status): Icon {
  switch (result) {
    case 'Accepted': return IconTrophy;
    case 'Wrong Answer': return IconX;
    default: return IconBug;
  }
}

useHead({
  title: "主页 | RustOJ"
})
</script>

<style scoped>
.sider {
  height: 100%;
  border-right: 1.5px solid #ffffff17;
}
</style>