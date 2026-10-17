<script setup lang="ts">
defineProps<{ message?: string }>();
const emit = defineEmits<{ dismiss: [] }>();
</script>

<template>
    <Transition name="pg-toast">
        <div v-if="message" class="pg-toast" role="alert">
            <span>{{ message }}</span>
            <button type="button" aria-label="Dismiss" @click="emit('dismiss')">×</button>
        </div>
    </Transition>
</template>

<style scoped>
.pg-toast {
    position: fixed;
    right: 24px;
    bottom: 24px;
    z-index: 100;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    max-width: min(420px, calc(100vw - 48px));
    padding: 12px 16px;
    border: 1px solid var(--vp-c-warning-2);
    border-radius: var(--pg-radius);
    background: var(--vp-c-bg-elv);
    box-shadow: var(--vp-shadow-3);
    color: var(--vp-c-text-1);
    font-size: 14px;
}

.pg-toast button {
    color: var(--vp-c-text-2);
    font-size: 18px;
    line-height: 1;
}

.pg-toast-enter-active,
.pg-toast-leave-active {
    transition:
        opacity 0.2s,
        transform 0.2s;
}

.pg-toast-enter-from,
.pg-toast-leave-to {
    opacity: 0;
    transform: translateY(8px);
}
</style>
