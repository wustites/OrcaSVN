import { createRouter, createWebHashHistory } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue'
import WorkspaceView from '@/views/WorkspaceView.vue'
import CheckoutView from '@/views/CheckoutView.vue'
import CommitView from '@/views/CommitView.vue'
import LogView from '@/views/LogView.vue'
import DiffView from '@/views/DiffView.vue'
import BlameView from '@/views/BlameView.vue'
import SettingsView from '@/views/SettingsView.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      component: MainLayout,
      children: [
        {
          path: '',
          name: 'workspace',
          component: WorkspaceView,
        },
        {
          path: 'checkout',
          name: 'checkout',
          component: CheckoutView,
        },
        {
          path: 'commit',
          name: 'commit',
          component: CommitView,
        },
        {
          path: 'log',
          name: 'log',
          component: LogView,
        },
        {
          path: 'diff',
          name: 'diff',
          component: DiffView,
        },
        {
          path: 'blame',
          name: 'blame',
          component: BlameView,
        },
        {
          path: 'settings',
          name: 'settings',
          component: SettingsView,
        },
      ],
    },
  ],
})

export default router
