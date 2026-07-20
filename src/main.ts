import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { ElAlert } from 'element-plus/es/components/alert/index'
import { ElAside, ElContainer, ElHeader, ElMain } from 'element-plus/es/components/container/index'
import { ElBreadcrumb, ElBreadcrumbItem } from 'element-plus/es/components/breadcrumb/index'
import { ElButton } from 'element-plus/es/components/button/index'
import { ElCard } from 'element-plus/es/components/card/index'
import { ElConfigProvider } from 'element-plus/es/components/config-provider/index'
import { ElDatePicker } from 'element-plus/es/components/date-picker/index'
import { ElDescriptions, ElDescriptionsItem } from 'element-plus/es/components/descriptions/index'
import { ElDialog } from 'element-plus/es/components/dialog/index'
import { ElDivider } from 'element-plus/es/components/divider/index'
import {
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
} from 'element-plus/es/components/dropdown/index'
import { ElEmpty } from 'element-plus/es/components/empty/index'
import { ElForm, ElFormItem } from 'element-plus/es/components/form/index'
import { ElIcon } from 'element-plus/es/components/icon/index'
import { ElInput } from 'element-plus/es/components/input/index'
import { ElInputNumber } from 'element-plus/es/components/input-number/index'
import { ElLoading } from 'element-plus/es/components/loading/index'
import { ElMenu, ElMenuItem } from 'element-plus/es/components/menu/index'
import { ElOption, ElSelect } from 'element-plus/es/components/select/index'
import {
  ElRadio,
  ElRadioButton,
  ElRadioGroup,
} from 'element-plus/es/components/radio/index'
import { ElSkeleton } from 'element-plus/es/components/skeleton/index'
import { ElSwitch } from 'element-plus/es/components/switch/index'
import { ElTable, ElTableColumn } from 'element-plus/es/components/table/index'
import { ElTag } from 'element-plus/es/components/tag/index'
import { ElTooltip } from 'element-plus/es/components/tooltip/index'
import 'element-plus/dist/index.css'
import 'element-plus/theme-chalk/dark/css-vars.css'
import 'material-symbols/rounded.css'
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  Back,
  Brush,
  Calendar,
  ChatLineSquare,
  Check,
  CircleCheck,
  Close,
  Connection,
  CopyDocument,
  Delete,
  Document,
  Download,
  Edit,
  Folder,
  FolderOpened,
  HomeFilled,
  InfoFilled,
  Link,
  List,
  Loading,
  Minus,
  Monitor,
  Moon,
  Plus,
  PriceTag,
  Refresh,
  RefreshLeft,
  RefreshRight,
  Setting,
  Search,
  Sunny,
  Switch,
  Upload,
  User,
} from '@/components/icons/materialIcons'
import router from './router'
import i18n from './i18n'
import App from './App.vue'
import { initializeTheme } from './composables/useTheme'
import './style.css'

initializeTheme()

const app = createApp(App)
const pinia = createPinia()

const components = [
  ElAlert,
  ElAside,
  ElBreadcrumb,
  ElBreadcrumbItem,
  ElButton,
  ElCard,
  ElConfigProvider,
  ElContainer,
  ElDatePicker,
  ElDescriptions,
  ElDescriptionsItem,
  ElDialog,
  ElDivider,
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElHeader,
  ElIcon,
  ElInput,
  ElInputNumber,
  ElMain,
  ElMenu,
  ElMenuItem,
  ElOption,
  ElRadio,
  ElRadioButton,
  ElRadioGroup,
  ElSelect,
  ElSkeleton,
  ElSwitch,
  ElTable,
  ElTableColumn,
  ElTag,
  ElTooltip,
]

const icons = {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  Back,
  Brush,
  Calendar,
  ChatLineSquare,
  Check,
  CircleCheck,
  Close,
  Connection,
  CopyDocument,
  Delete,
  Document,
  Download,
  Edit,
  Folder,
  FolderOpened,
  HomeFilled,
  InfoFilled,
  Link,
  List,
  Loading,
  Minus,
  Monitor,
  Moon,
  Plus,
  PriceTag,
  Refresh,
  RefreshLeft,
  RefreshRight,
  Setting,
  Search,
  Sunny,
  Switch,
  Upload,
  User,
}

for (const component of components) {
  app.use(component)
}

for (const [key, component] of Object.entries(icons)) {
  app.component(key, component)
}

app.use(pinia)
app.use(router)
app.use(i18n)
app.use(ElLoading)
app.mount('#app')
