<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, h, defineComponent } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { Calendar, Clock, FileText, Trash2, Plus, ChevronLeft, ChevronRight, CalendarDays, Edit3, X, Check, AlertCircle, ChevronDown } from 'lucide-vue-next';
import type { Task, Config } from './types';
import { toastState, showToast, hideToast } from './main';

const currentDate = ref(new Date());
const selectedDate = ref(new Date());
const tasks = ref<Task[]>([]);
const showEditDialog = ref(false);
const editingTask = ref<Task | null>(null);
const loading = ref(false);
const deletingId = ref<string | null>(null);

const weekDays = ['一', '二', '三', '四', '五', '六', '日'];

const currentYear = computed(() => currentDate.value.getFullYear());
const currentMonth = computed(() => currentDate.value.getMonth() + 1);

const taskTypeLookup = computed(() => {
  const dailySet = new Set(tasks.value.filter(t => t.task_type === 'daily').map(t => t.id));
  const weeklyByDay = new Map<number, Set<string>>();
  const onceByDate = new Map<string, Set<string>>();

  for (const t of tasks.value) {
    if (t.task_type === 'weekly' && t.days_of_week) {
      for (const d of t.days_of_week) {
        let daySet = weeklyByDay.get(d);
        if (!daySet) {
          daySet = new Set();
          weeklyByDay.set(d, daySet);
        }
        daySet.add(t.id);
      }
    }
    if (t.task_type === 'once' && t.once_date) {
      let dateSet = onceByDate.get(t.once_date);
      if (!dateSet) {
        dateSet = new Set();
        onceByDate.set(t.once_date, dateSet);
      }
      dateSet.add(t.id);
    }
  }

  return {
    daily: dailySet,
    weekly: weeklyByDay,
    once: onceByDate,
    hasAny: dailySet.size > 0 || weeklyByDay.size > 0 || onceByDate.size > 0
  };
});

const calendarDays = computed(() => {
  const year = currentYear.value;
  const month = currentMonth.value;
  const firstDay = new Date(year, month - 1, 1);
  const lastDay = new Date(year, month, 0);
  const startWeekday = (firstDay.getDay() + 6) % 7;
  const days: { date: Date; isCurrentMonth: boolean; taskTypes: string[] }[] = [];

  const lookup = taskTypeLookup.value;

  for (let i = 0; i < startWeekday; i++) {
    const d = new Date(year, month - 1, -startWeekday + i + 1);
    days.push({ date: d, isCurrentMonth: false, taskTypes: [] });
  }

  for (let i = 1; i <= lastDay.getDate(); i++) {
    const d = new Date(year, month - 1, i);
    const dow = (d.getDay() + 6) % 7 + 1;
    const dateStr = formatDate(d);
    const taskTypes: string[] = [];
    if (lookup.daily.size > 0) taskTypes.push('daily');
    const weekSet = lookup.weekly.get(dow);
    if (weekSet && weekSet.size > 0) taskTypes.push('weekly');
    const onceSet = lookup.once.get(dateStr);
    if (onceSet && onceSet.size > 0) taskTypes.push('once');
    days.push({ date: d, isCurrentMonth: true, taskTypes });
  }

  const remaining = 42 - days.length;
  for (let i = 1; i <= remaining; i++) {
    const d = new Date(year, month, i);
    days.push({ date: d, isCurrentMonth: false, taskTypes: [] });
  }

  return days;
});

const selectedDateTasks = computed(() => {
  const dateStr = formatDate(selectedDate.value);
  return tasks.value.filter(t => {
    if (t.task_type === 'daily') return true;
    if (t.task_type === 'weekly' && t.days_of_week) {
      const wd = (selectedDate.value.getDay() + 6) % 7 + 1;
      return t.days_of_week.includes(wd);
    }
    if (t.task_type === 'once' && t.once_date) {
      return t.once_date === dateStr;
    }
    return false;
  }).sort((a, b) => a.time.localeCompare(b.time));
});

const defaultCliConfig = ref<Config>({ command: 'claude', args: [{ name: '-p', value: '' }, { name: '--permission-mode', value: 'acceptEdits' }] });

function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function formatDisplayDate(d: Date): string {
  const y = d.getFullYear();
  const m = d.getMonth() + 1;
  const day = d.getDate();
  return `${y}年${m}月${day}日`;
}

function isToday(d: Date): boolean {
  const t = new Date();
  return d.getDate() === t.getDate() &&
    d.getMonth() === t.getMonth() &&
    d.getFullYear() === t.getFullYear();
}

function isSelected(d: Date): boolean {
  return formatDate(d) === formatDate(selectedDate.value);
}

function prevMonth() {
  currentDate.value = new Date(currentYear.value, currentMonth.value - 2, 1);
}

function nextMonth() {
  currentDate.value = new Date(currentYear.value, currentMonth.value, 1);
}

function goToday() {
  const now = new Date();
  currentDate.value = new Date(now.getFullYear(), now.getMonth(), 1);
  selectedDate.value = now;
}

function selectDay(day: { date: Date; isCurrentMonth: boolean }) {
  if (day.isCurrentMonth) {
    selectedDate.value = day.date;
  }
}

function openAddDialog() {
  editingTask.value = null;
  showEditDialog.value = true;
}

function openEditDialog(task: Task) {
  editingTask.value = { ...task };
  showEditDialog.value = true;
}

async function loadTasks() {
  loading.value = true;
  try {
    tasks.value = await invoke<Task[]>('get_tasks');
  } catch (e) {
    console.error('Failed to load tasks:', e);
  } finally {
    loading.value = false;
  }
}

async function loadConfig() {
  try {
    return await invoke<Config>('get_config');
  } catch (e) {
    console.error('Failed to load config:', e);
    return { command: 'claude', args: [{ name: '-p', value: '' }, { name: '--permission-mode', value: 'acceptEdits' }] };
  }
}

async function deleteTask(id: string) {
  if (deletingId.value === id) {
    try {
      await invoke('delete_task', { id });
      deletingId.value = null;
      await loadTasks();
      showToast('任务已删除', 'success');
    } catch (e) {
      console.error('Failed to delete task:', e);
      showToast('删除失败', 'error');
    }
  } else {
    deletingId.value = id;
  }
}

function cancelDelete() {
  deletingId.value = null;
}

function truncatePrompt(prompt: string, maxLen = 24): string {
  if (prompt.length <= maxLen) return prompt;
  return prompt.substring(0, maxLen) + '...';
}

function getTaskTypeName(type: string): string {
  switch (type) {
    case 'daily': return '每日';
    case 'weekly': return '每周';
    case 'once': return '一次性';
    default: return type;
  }
}

function getTaskTypeColor(type: string): string {
  switch (type) {
    case 'daily': return '#10b981';
    case 'weekly': return '#f59e0b';
    case 'once': return '#0ea5e9';
    default: return '#94a3b8';
  }
}

let unlistenTasksChanged: UnlistenFn | null = null;

onMounted(async () => {
  loadTasks();
  defaultCliConfig.value = await loadConfig();
  goToday();
  document.addEventListener('contextmenu', e => e.preventDefault());

  unlistenTasksChanged = await listen('tasks-changed', () => {
    loadTasks();
  });
});

onUnmounted(() => {
  if (unlistenTasksChanged) {
    unlistenTasksChanged();
  }
});

const TaskEditDialog = defineComponent({
  name: 'TaskEditDialog',
  props: {
    task: { type: Object as () => Task | null, default: null },
    selectedDate: { type: String, default: '' }
  },
  emits: ['close', 'saved'],
  setup(props, { emit }) {
    const taskType = ref(props.task?.task_type || 'daily');
    const time = ref(props.task?.time || '08:00');
    const daysOfWeek = ref<number[]>(props.task?.days_of_week || []);
    const onceDate = ref(props.task?.once_date || props.selectedDate);
    const prompt = ref(props.task?.prompt || '');
    const label = ref(props.task?.label || '');
    const saving = ref(false);

    const cliExpanded = ref(false);
    const useGlobalDefault = ref(!props.task?.cli_command);
    const cliExe = ref(props.task?.cli_command?.exe || 'claude');
    const cliArgs = ref<{name: string; value: string}[]>(
      props.task?.cli_command?.args || [
        { name: '-p', value: '' },
        { name: '--permission-mode', value: 'acceptEdits' }
      ]
    );

    const weekDaysList = [
      { num: 1, name: '一' },
      { num: 2, name: '二' },
      { num: 3, name: '三' },
      { num: 4, name: '四' },
      { num: 5, name: '五' },
      { num: 6, name: '六' },
      { num: 7, name: '日' }
    ];

    const hours = Array.from({ length: 24 }, (_, i) => i);
    const minutes = Array.from({ length: 60 }, (_, i) => i);

    const isEditing = computed(() => !!props.task);

    const isFormValid = computed(() => {
      if (!prompt.value.trim()) return false;
      if (taskType.value === 'weekly' && daysOfWeek.value.length === 0) return false;
      return true;
    });

    const defaultConfig = computed(() => defaultCliConfig.value);

    const commandPreview = computed(() => {
      const exe = useGlobalDefault.value ? defaultConfig.value.command : cliExe.value;
      const args = useGlobalDefault.value ? defaultConfig.value.args : cliArgs.value;
      const parts: string[] = [exe];
      for (const arg of args) {
        parts.push(arg.name);
        if (arg.value) {
          parts.push(arg.value);
        }
      }
      parts.push(prompt.value || '<检索主题>');
      return parts.join(' ');
    });

    function toggleDay(num: number) {
      const idx = daysOfWeek.value.indexOf(num);
      if (idx >= 0) {
        daysOfWeek.value.splice(idx, 1);
      } else {
        daysOfWeek.value.push(num);
        daysOfWeek.value.sort();
      }
    }

    function addArg() {
      cliArgs.value.push({ name: '', value: '' });
    }

    function removeArg(index: number) {
      cliArgs.value.splice(index, 1);
    }

    async function save() {
      if (!prompt.value.trim()) {
        showToast('检索主题不能为空', 'error');
        return;
      }

      saving.value = true;

      if (taskType.value === 'weekly' && daysOfWeek.value.length === 0) {
        showToast('请至少选择一个星期', 'error');
        saving.value = false;
        return;
      }

      if (!prompt.value.trim()) {
        showToast('请输入检索主题', 'error');
        saving.value = false;
        return;
      }

      let cliCommand = null;
      if (!useGlobalDefault.value) {
        cliCommand = {
          exe: cliExe.value,
          args: cliArgs.value.filter(a => a.name.trim() !== '')
        };
      }

      const taskData: Task = {
        id: props.task?.id || '',
        task_type: taskType.value,
        time: time.value,
        days_of_week: taskType.value === 'weekly' ? daysOfWeek.value : null,
        once_date: taskType.value === 'once' ? onceDate.value : null,
        label: label.value || null,
        prompt: prompt.value,
        cli_command: cliCommand,
        next_trigger: props.task?.next_trigger || ''
      };

      try {
        if (isEditing.value) {
          await invoke('update_task', { task: taskData });
        } else {
          await invoke('add_task', { task: taskData });
        }
        showToast(isEditing.value ? '任务已更新' : '任务已添加', 'success');
        emit('saved');
      } catch (e) {
        console.error('Failed to save task:', e);
        showToast('保存失败', 'error');
      } finally {
        saving.value = false;
      }
    }

    function close() {
      emit('close');
    }

    return () => h('div', { class: 'dialog-overlay' }, [
      h('div', { class: 'dialog' }, [
        h('div', { class: 'dialog-title' }, [
          h('span', {}, isEditing.value ? '编辑任务' : '添加任务')
        ]),
        h('div', { class: 'dialog-content' }, [
          h('div', { class: 'form-row' }, [
            h('label', {}, '任务类型'),
            h('div', { class: 'radio-group' }, [
              h('label', { class: 'radio-item' }, [
                h('input', { type: 'radio', value: 'daily', checked: taskType.value === 'daily', onChange: (e: Event) => taskType.value = (e.target as HTMLInputElement).value }),
                h('span', { class: 'radio-label' }, '每天')
              ]),
              h('label', { class: 'radio-item' }, [
                h('input', { type: 'radio', value: 'weekly', checked: taskType.value === 'weekly', onChange: (e: Event) => taskType.value = (e.target as HTMLInputElement).value }),
                h('span', { class: 'radio-label' }, '每周')
              ]),
              h('label', { class: 'radio-item' }, [
                h('input', { type: 'radio', value: 'once', checked: taskType.value === 'once', onChange: (e: Event) => taskType.value = (e.target as HTMLInputElement).value }),
                h('span', { class: 'radio-label' }, '一次性')
              ])
            ])
          ]),
          h('div', { class: 'form-row' }, [
            h('label', {}, '触发时间'),
            h('div', { class: 'time-selectors' }, [
              h('select', {
                value: time.value.split(':')[0],
                onChange: (e: Event) => {
                  const newHour = (e.target as HTMLSelectElement).value;
                  time.value = newHour + ':' + time.value.split(':')[1];
                }
              }, hours.map(hr => h('option', { value: String(hr).padStart(2, '0') }, String(hr).padStart(2, '0')))),
              h('span', { class: 'time-separator' }, ':'),
              h('select', {
                value: time.value.split(':')[1],
                onChange: (e: Event) => {
                  const newMin = (e.target as HTMLSelectElement).value;
                  time.value = time.value.split(':')[0] + ':' + newMin;
                }
              }, minutes.map(mn => h('option', { value: String(mn).padStart(2, '0') }, String(mn).padStart(2, '0'))))
            ])
          ]),
          taskType.value === 'weekly' && h('div', { class: 'form-row' }, [
            h('label', {}, '每周重复'),
            h('div', { class: 'checkbox-group' }, [
              ...weekDaysList.map(d => h('label', { class: 'checkbox-item' }, [
                h('input', { type: 'checkbox', checked: daysOfWeek.value.includes(d.num), onChange: () => toggleDay(d.num) }),
                h('span', {}, d.name)
              ]))
            ])
          ]),
          taskType.value === 'once' && h('div', { class: 'form-row' }, [
            h('label', {}, '一次性日期'),
            h('input', { type: 'date', value: onceDate.value, onChange: (e: Event) => onceDate.value = (e.target as HTMLInputElement).value })
          ]),
          h('div', { class: 'form-row' }, [
            h('label', {}, [
              '检索主题 ',
              h('span', { class: 'required' }, '*')
            ]),
            h('textarea', {
              value: prompt.value,
              placeholder: '输入检索主题...',
              rows: 3,
              onInput: (e: Event) => prompt.value = (e.target as HTMLTextAreaElement).value
            })
          ]),
          h('div', { class: 'form-row' }, [
            h('label', {}, '备注'),
            h('input', {
              type: 'text',
              value: label.value,
              placeholder: '可选备注',
              onInput: (e: Event) => label.value = (e.target as HTMLInputElement).value
            })
          ]),
          h('div', { class: 'cli-config-section' }, [
            h('div', {
              class: 'cli-config-header',
              onClick: () => cliExpanded.value = !cliExpanded.value,
              role: 'button',
              'aria-expanded': cliExpanded.value
            }, [
              h(ChevronDown, { class: 'cli-expand-icon', size: 14, style: { transform: cliExpanded.value ? 'rotate(180deg)' : 'rotate(0deg)', transition: 'transform 200ms ease' } }),
              h('span', {}, 'CLI 命令配置'),
              !useGlobalDefault.value && h('span', { class: 'cli-custom-badge' }, '自定义')
            ]),
            cliExpanded.value && h('div', { class: 'cli-config-body' }, [
              h('div', { class: 'form-row' }, [
                h('label', { class: 'checkbox-label' }, [
                  h('input', {
                    type: 'checkbox',
                    checked: useGlobalDefault.value,
                    onChange: (e: Event) => useGlobalDefault.value = (e.target as HTMLInputElement).checked
                  }),
                  h('span', {}, '使用全局默认')
                ])
              ]),
              h('div', { class: 'cli-fields', style: { display: useGlobalDefault.value ? 'none' : 'block' } }, [
                h('div', { class: 'form-row' }, [
                  h('label', {}, '命令'),
                  h('input', {
                    type: 'text',
                    value: cliExe.value,
                    placeholder: 'claude',
                    onInput: (e: Event) => cliExe.value = (e.target as HTMLInputElement).value
                  })
                ]),
                h('div', { class: 'form-row' }, [
                  h('label', {}, 'Args'),
                  h('div', { class: 'cli-args-list' }, [
                    ...cliArgs.value.map((arg, index) => h('div', { class: 'cli-arg-row', key: index }, [
                      h('input', {
                        type: 'text',
                        class: 'cli-arg-name',
                        value: arg.name,
                        placeholder: '--flag',
                        onInput: (e: Event) => {
                          const target = e.target as HTMLInputElement;
                          cliArgs.value[index].name = target.value;
                        }
                      }),
                      h('input', {
                        type: 'text',
                        class: 'cli-arg-value',
                        value: arg.value,
                        placeholder: 'value',
                        onInput: (e: Event) => {
                          const target = e.target as HTMLInputElement;
                          cliArgs.value[index].value = target.value;
                        }
                      }),
                      h('button', {
                        class: 'cli-arg-remove',
                        onClick: () => removeArg(index),
                        'aria-label': '删除参数'
                      }, h(X, { size: 14 }))
                    ])),
                    h('button', {
                      class: 'cli-arg-add',
                      onClick: addArg
                    }, [h(Plus, { size: 12 }), ' 添加参数'])
                  ])
                ])
              ]),
              h('div', { class: 'cli-preview' }, [
                h('span', { class: 'cli-preview-label' }, '预览: '),
                h('code', { class: 'cli-preview-command' }, commandPreview.value)
              ])
            ])
          ])
        ]),
        h('div', { class: 'dialog-buttons' }, [
          h('button', {
            class: 'btn-secondary',
            onClick: close,
            disabled: saving.value
          }, '取消'),
          h('button', {
            class: 'btn-primary',
            onClick: save,
            disabled: saving.value || !isFormValid.value
          }, saving.value ? '保存中...' : '确定')
        ])
      ])
    ]);
  }
});
</script>

<template>
  <div class="app">
    <div class="main-container">
      <div class="left-panel">
        <header class="header">
          <button class="nav-btn" @click="prevMonth" aria-label="上一月">
            <ChevronLeft :size="16" />
          </button>
          <span class="month-label">{{ currentYear }}年{{ currentMonth }}月</span>
          <button class="nav-btn" @click="nextMonth" aria-label="下一月">
            <ChevronRight :size="16" />
          </button>
          <button class="today-btn" @click="goToday">今天</button>
        </header>

        <div class="weekdays">
          <span v-for="day in weekDays" :key="day" class="weekday">{{ day }}</span>
        </div>

        <div class="calendar">
          <div
            v-for="(day, index) in calendarDays"
            :key="index"
            class="day"
            :class="{
              'other-month': !day.isCurrentMonth,
              'today': isToday(day.date),
              'selected': isSelected(day.date)
            }"
            @click="selectDay(day)"
          >
            <span class="day-number">{{ day.date.getDate() }}</span>
            <span v-if="day.taskTypes.length && day.isCurrentMonth" class="task-dots">
              <span
                v-for="(tt, idx) in day.taskTypes"
                :key="idx"
                class="task-dot"
                :style="{ background: getTaskTypeColor(tt) }"
              ></span>
            </span>
          </div>
        </div>
      </div>

      <div class="right-panel">
        <div class="selected-date">
          <CalendarDays :size="14" class="date-icon" />
          {{ formatDisplayDate(selectedDate) }}
        </div>

        <div class="task-list" v-if="selectedDateTasks.length > 0">
          <div
            v-for="task in selectedDateTasks"
            :key="task.id"
            class="task-item"
          >
            <div class="task-header">
              <span class="task-dot" :style="{ background: getTaskTypeColor(task.task_type) }"></span>
              <Clock :size="12" class="task-time-icon" />
              <span class="task-time">{{ task.time }}</span>
              <span class="task-type" :style="{ color: getTaskTypeColor(task.task_type) }">{{ getTaskTypeName(task.task_type) }}</span>
            </div>
            <div class="task-label" v-if="task.label">
              <FileText :size="11" />
              {{ task.label }}
            </div>
            <div class="task-prompt" :title="task.prompt">"{{ truncatePrompt(task.prompt, 28) }}"</div>
            <div class="task-actions">
              <button
                class="action-btn edit-btn"
                @click="openEditDialog(task)"
                aria-label="编辑任务"
              >
                <Edit3 :size="13" />
              </button>
              <template v-if="deletingId === task.id">
                <button
                  class="action-btn confirm-delete-btn"
                  @click="deleteTask(task.id)"
                  aria-label="确认删除"
                >
                  <Check :size="13" />
                </button>
                <button
                  class="action-btn cancel-btn"
                  @click="cancelDelete"
                  aria-label="取消删除"
                >
                  <X :size="13" />
                </button>
              </template>
              <template v-else>
                <button
                  class="action-btn delete-btn"
                  @click="deleteTask(task.id)"
                  aria-label="删除任务"
                >
                  <Trash2 :size="13" />
                </button>
              </template>
            </div>
          </div>
        </div>

        <div class="no-tasks" v-else>
          <Calendar :size="32" class="empty-icon" />
          <span>暂无任务</span>
        </div>

        <button class="add-btn" @click="openAddDialog">
          <Plus :size="16" />
          添加任务
        </button>
      </div>
    </div>

    <TaskEditDialog
      v-if="showEditDialog"
      :task="editingTask"
      :selected-date="formatDate(selectedDate)"
      @close="showEditDialog = false"
      @saved="showEditDialog = false; loadTasks()"
    />

    <Transition name="toast">
      <div v-if="toastState.visible" class="toast" :class="toastState.type">
        <AlertCircle v-if="toastState.type === 'error'" :size="16" />
        <Check v-else-if="toastState.type === 'success'" :size="16" />
        <span v-else :size="16" />
        <span class="toast-message">{{ toastState.message }}</span>
        <button class="toast-close" @click="hideToast">
          <X :size="14" />
        </button>
      </div>
    </Transition>
  </div>
</template>

<style>
:root {
  --color-primary: #0d9488;
  --color-primary-hover: #0f766e;
  --color-primary-light: #f0fdfa;
  --color-secondary: #14b8a6;
  --color-accent: #ea580c;
  --color-background: #fafbfc;
  --color-surface: #ffffff;
  --color-foreground: #134e4a;
  --color-muted: #e8f1f4;
  --color-border: #99f6e4;
  --color-text: #1e293b;
  --color-text-secondary: #64748b;
  --color-text-muted: #94a3b8;
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-info: #0ea5e9;
  --font-family: 'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.08);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.12);
  --transition-fast: 150ms ease;
  --transition-normal: 250ms ease;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: var(--font-family);
  font-size: 13px;
  color: var(--color-text);
  background: var(--color-background);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.app {
  width: 480px;
  height: 380px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--color-background);
}

.main-container {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.left-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 10px 12px;
  border-right: 1px solid var(--color-border);
  background: var(--color-surface);
}

.header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 10px;
}

.nav-btn {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 6px 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text);
  transition: all var(--transition-fast);
}

.nav-btn:hover {
  background: var(--color-primary-light);
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.nav-btn:active {
  transform: scale(0.95);
}

.month-label {
  font-weight: 600;
  flex: 1;
  font-size: 15px;
  color: var(--color-foreground);
  letter-spacing: -0.01em;
}

.today-btn {
  background: var(--color-primary-light);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 5px 10px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  color: var(--color-primary);
  transition: all var(--transition-fast);
}

.today-btn:hover {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.today-btn:active {
  transform: scale(0.97);
}

.weekdays {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  text-align: center;
  padding: 6px 0;
  background: var(--color-muted);
  border-radius: var(--radius-md);
  margin-bottom: 6px;
}

.weekday {
  color: var(--color-text-secondary);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.calendar {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 3px;
  flex: 1;
}

.day {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  padding: 3px;
  cursor: pointer;
  border-radius: var(--radius-md);
  min-height: 26px;
  position: relative;
  background: var(--color-surface);
  transition: all var(--transition-fast);
}

.day:hover {
  background: var(--color-primary-light);
}

.day.other-month {
  background: transparent;
}

.day.other-month .day-number {
  color: var(--color-text-muted);
}

.day.today {
  background: #dcfce7;
}

.day.today .day-number {
  color: var(--color-success);
  font-weight: 700;
}

.day.selected {
  background: var(--color-primary);
}

.day.selected:hover {
  background: var(--color-primary);
}

.day.selected .day-number {
  color: white;
}

.day.selected .task-dots .task-dot {
  background: white;
}

.day-number {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text);
}

.task-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  position: absolute;
  bottom: 3px;
  background: var(--color-primary);
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.6; transform: scale(0.8); }
}

.right-panel {
  width: 168px;
  display: flex;
  flex-direction: column;
  padding: 10px 12px;
  background: var(--color-background);
}

.selected-date {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-foreground);
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  gap: 5px;
}

.date-icon {
  color: var(--color-primary);
}

.task-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.task-list::-webkit-scrollbar {
  width: 4px;
}

.task-list::-webkit-scrollbar-track {
  background: transparent;
}

.task-list::-webkit-scrollbar-thumb {
  background: var(--color-border);
  border-radius: 2px;
}

.task-item {
  background: var(--color-surface);
  border-radius: var(--radius-md);
  padding: 10px;
  box-shadow: var(--shadow-sm);
  border: 1px solid transparent;
  transition: all var(--transition-fast);
}

.task-item:hover {
  border-color: var(--color-border);
  box-shadow: var(--shadow-md);
}

.task-header {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 4px;
}

.task-dots {
  display: flex;
  gap: 2px;
}

.task-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.task-time-icon {
  color: var(--color-text-muted);
}

.task-time {
  font-weight: 700;
  font-size: 12px;
  color: var(--color-text);
}

.task-type {
  font-size: 10px;
  font-weight: 600;
  margin-left: auto;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.task-label {
  font-size: 11px;
  color: var(--color-text-secondary);
  margin-bottom: 3px;
  display: flex;
  align-items: center;
  gap: 3px;
}

.task-prompt {
  font-size: 10px;
  color: var(--color-text-muted);
  line-height: 1.4;
  margin-bottom: 6px;
  cursor: default;
}

.task-actions {
  display: flex;
  justify-content: flex-end;
  gap: 4px;
}

.action-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.5;
  transition: all var(--transition-fast);
}

.action-btn:hover {
  opacity: 1;
}

.action-btn:active {
  transform: scale(0.92);
}

.edit-btn {
  color: var(--color-primary);
}

.edit-btn:hover {
  background: var(--color-primary-light);
}

.delete-btn {
  color: var(--color-error);
}

.delete-btn:hover {
  background: #fef2f2;
}

.confirm-delete-btn {
  color: var(--color-success);
  background: #ecfdf5;
}

.confirm-delete-btn:hover {
  background: #d1fae5;
}

.cancel-btn {
  color: var(--color-text-muted);
  background: var(--color-muted);
}

.cancel-btn:hover {
  background: #e2e8f0;
}

.no-tasks {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--color-text-muted);
  font-size: 12px;
}

.empty-icon {
  opacity: 0.4;
}

.add-btn {
  margin-top: 10px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: var(--radius-md);
  padding: 10px 14px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all var(--transition-fast);
  box-shadow: 0 2px 8px rgba(13, 148, 136, 0.3);
}

.add-btn:hover {
  background: var(--color-primary-hover);
  box-shadow: 0 4px 12px rgba(13, 148, 136, 0.4);
}

.add-btn:active {
  transform: scale(0.97);
}

.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(15, 23, 42, 0.6);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background: var(--color-surface);
  border-radius: var(--radius-lg);
  width: 320px;
  max-height: 90vh;
  overflow-y: auto;
  box-shadow: var(--shadow-lg);
  animation: dialogIn 250ms ease;
}

@keyframes dialogIn {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.dialog-title {
  padding: 16px 20px;
  font-weight: 700;
  font-size: 15px;
  border-bottom: 1px solid var(--color-muted);
  color: var(--color-foreground);
}

.dialog-content {
  padding: 16px 20px;
}

.form-row {
  margin-bottom: 14px;
}

.form-row > label:first-child {
  display: block;
  margin-bottom: 6px;
  font-weight: 600;
  font-size: 12px;
  color: var(--color-text);
}

.required {
  color: var(--color-error);
}

.radio-group {
  display: flex;
  gap: 8px;
}

.radio-item {
  display: flex;
  align-items: center;
  gap: 5px;
  cursor: pointer;
  padding: 6px 10px;
  background: var(--color-muted);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
  font-size: 12px;
}

.radio-item:has(input:checked) {
  background: var(--color-primary-light);
  color: var(--color-primary);
}

.radio-item input {
  display: none;
}

.radio-label {
  font-weight: 500;
}

.time-selectors {
  display: flex;
  align-items: center;
  gap: 6px;
}

.time-selectors select {
  padding: 7px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 600;
  background: var(--color-surface);
  color: var(--color-text);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.time-selectors select:hover {
  border-color: var(--color-primary);
}

.time-selectors select:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.15);
}

.time-separator {
  font-weight: 700;
  color: var(--color-text-secondary);
}

.checkbox-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.checkbox-item {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  background: var(--color-muted);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
  transition: all var(--transition-fast);
}

.checkbox-item:has(input:checked) {
  background: var(--color-primary);
  color: white;
}

.checkbox-item input {
  display: none;
}

input[type="text"],
input[type="date"],
textarea,
select {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: 13px;
  font-family: var(--font-family);
  transition: all var(--transition-fast);
}

input[type="text"]:focus,
input[type="date"]:focus,
textarea:focus,
select:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.15);
}

textarea {
  resize: vertical;
  min-height: 70px;
  line-height: 1.5;
}

.dialog-buttons {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 20px;
  border-top: 1px solid var(--color-muted);
}

.btn-primary,
.btn-secondary {
  padding: 8px 18px;
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  border: none;
}

.btn-primary {
  background: var(--color-primary);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-hover);
}

.btn-primary:active:not(:disabled) {
  transform: scale(0.97);
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-secondary {
  background: var(--color-muted);
  color: var(--color-text);
}

.btn-secondary:hover:not(:disabled) {
  background: #e2e8f0;
}

.btn-secondary:active:not(:disabled) {
  transform: scale(0.97);
}

.btn-secondary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.toast {
  position: fixed;
  top: 12px;
  right: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 500;
  box-shadow: var(--shadow-lg);
  z-index: 9999;
  min-width: 180px;
  max-width: 300px;
}

.toast.success {
  background: #ecfdf5;
  color: #065f46;
  border: 1px solid #a7f3d0;
}

.toast.error {
  background: #fef2f2;
  color: #991b1b;
  border: 1px solid #fecaca;
}

.toast.info {
  background: var(--color-primary-light);
  color: var(--color-foreground);
  border: 1px solid var(--color-border);
}

.toast-message {
  flex: 1;
  line-height: 1.4;
}

.toast-close {
  flex-shrink: 0;
  background: none;
  border: none;
  cursor: pointer;
  opacity: 0.6;
  padding: 2px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  transition: opacity var(--transition-fast);
}

.toast-close:hover {
  opacity: 1;
}

.toast-enter-active,
.toast-leave-active {
  transition: all var(--transition-normal);
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(20px);
}

.cli-config-section {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px dashed var(--color-border);
}

.cli-config-header {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  color: var(--color-text-secondary);
  font-size: 12px;
  font-weight: 600;
  user-select: none;
  transition: all var(--transition-fast);
  padding: 6px 8px;
  border-radius: var(--radius-sm);
}

.cli-config-header:hover {
  color: var(--color-primary);
  background: var(--color-primary-light);
}

.cli-expand-icon {
  flex-shrink: 0;
  color: var(--color-text-muted);
}

.cli-config-body {
  margin-top: 10px;
  padding: 10px;
  background: var(--color-muted);
  border-radius: var(--radius-md);
}

.cli-fields {
  margin-top: 10px;
}

.cli-args-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.cli-arg-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.cli-arg-name {
  flex: 1;
  min-width: 80px;
  padding: 6px 8px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 12px;
  background: var(--color-surface);
  font-family: monospace;
}

.cli-arg-name:focus {
  outline: none;
  border-color: var(--color-primary);
}

.cli-arg-value {
  flex: 1.5;
  min-width: 100px;
  padding: 6px 8px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 12px;
  background: var(--color-surface);
  font-family: monospace;
}

.cli-arg-value:focus {
  outline: none;
  border-color: var(--color-primary);
}

.cli-arg-remove {
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  font-size: 16px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
  display: flex;
  align-items: center;
  justify-content: center;
}

.cli-arg-remove:hover {
  background: #fef2f2;
  color: var(--color-error);
}

.cli-arg-add {
  margin-top: 6px;
  padding: 6px 12px;
  border: 1px dashed var(--color-border);
  background: transparent;
  border-radius: var(--radius-sm);
  font-size: 12px;
  color: var(--color-primary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.cli-arg-add:hover {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
}

.cli-preview {
  margin-top: 10px;
  padding: 8px 10px;
  background: #f8fafc;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-family: monospace;
  word-break: break-all;
}

.cli-preview-label {
  color: var(--color-text-muted);
  margin-right: 6px;
}

.cli-preview-command {
  color: var(--color-text);
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] {
  width: auto;
  margin: 0;
}

.cli-custom-badge {
  margin-left: auto;
  padding: 2px 8px;
  background: var(--color-accent, #ea580c);
  color: white;
  border-radius: 10px;
  font-size: 10px;
  font-weight: 600;
}
</style>
