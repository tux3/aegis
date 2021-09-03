#ifndef AEGISK_MONITOR_H
#define AEGISK_MONITOR_H

int start_aegisc_monitor_thread(void);
void stop_aegisc_monitor_thread(void);
pid_t aegisc_umh_get_pid(void);

#endif // AEGISK_MONITOR_H
