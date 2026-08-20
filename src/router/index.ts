import { createRouter, createWebHashHistory } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue'

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
          component: () => import('@/views/WorkspaceView.vue'),
          meta: { title: 'menu.workspace' },
        },
        {
          path: 'checkout',
          name: 'checkout',
          component: () => import('@/views/CheckoutView.vue'),
          meta: { title: 'menu.checkout' },
        },
        {
          path: 'commit',
          name: 'commit',
          component: () => import('@/views/CommitView.vue'),
          meta: { title: 'menu.commit' },
        },
        {
          path: 'update',
          name: 'update',
          component: () => import('@/views/UpdateView.vue'),
          meta: { title: 'menu.update' },
        },
        {
          path: 'log',
          name: 'log',
          component: () => import('@/views/LogView.vue'),
          meta: { title: 'menu.log' },
        },
        {
          path: 'diff',
          name: 'diff',
          component: () => import('@/views/DiffView.vue'),
          meta: { title: 'menu.diff' },
        },
        {
          path: 'stash',
          name: 'stash',
          component: () => import('@/views/StashView.vue'),
          meta: { title: 'menu.stash' },
        },
        {
          path: 'blame',
          name: 'blame',
          component: () => import('@/views/BlameView.vue'),
          meta: { title: 'menu.blame' },
        },
        {
          path: 'settings',
          name: 'settings',
          component: () => import('@/views/SettingsView.vue'),
          meta: { title: 'nav.settings' },
        },
      ],
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: '/',
    },
  ],
})

export default router
