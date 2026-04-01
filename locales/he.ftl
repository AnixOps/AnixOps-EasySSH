### EasySSH - תרגום עברי (RTL)
### RTL Language - Right to Left Support

## מידע על האפליקציה
-app-name = EasySSH
-app-description = לקוח SSH מודרני לכל הפלטפורמות
-app-version = גרסה { $version }

## ממשק משתמש כללי
-general-ok = אישור
-general-cancel = ביטול
-general-save = שמירה
-general-delete = מחיקה
-general-edit = עריכה
-general-add = הוספה
-general-remove = הסרה
-general-close = סגירה
-general-back = חזרה
-general-next = הבא
-general-finish = סיום
-general-apply = החלה
-general-reset = איפוס
-general-search = חיפוש
-general-filter = סינון
-general-loading = טוען...
-general-empty = ריק
-general-error = שגיאה
-general-success = הצלחה
-general-warning = אזהרה
-general-info = מידע
-general-confirm = אישור
-general-yes = כן
-general-no = לא
-general-copy = העתקה
-general-paste = הדבקה
-general-cut = גזירה
-general-select-all = בחירת הכל
-general-undo = ביטול
-general-redo = ביצוע מחדש
-general-refresh = רענון
-general-reload = טעינה מחדש
-general-settings = הגדרות
-general-preferences = העדפות
-general-help = עזרה
-general-about = אודות
-general-feedback = משוב
-general-donate = תרומה
-general-exit = יציאה
-general-quit = יציאה

## חלון ראשי
-main-title = { -app-name }
-main-sidebar-servers = שרתים
-main-sidebar-groups = קבוצות
-main-sidebar-snippets = קטעי קוד
-main-sidebar-history = היסטוריה
-main-sidebar-settings = הגדרות
-main-sidebar-add-server = הוספת שרת
-main-sidebar-add-group = הוספת קבוצה
-main-sidebar-import = ייבוא
-main-sidebar-export = ייצוא
-main-status-ready = מוכן
-main-status-connecting = מתחבר...
-main-status-connected = מחובר
-main-status-disconnected = מנותק
-main-status-error = שגיאה
-main-status-syncing = מסנכרן...

## רשימת שרתים
-server-list-title = רשימת שרתים
-server-list-empty = אין שרתים מוגדרים
-server-list-empty-hint = לחץ על "הוספת שרת" כדי להתחיל
-server-list-search-placeholder = חיפוש שרתים...
-server-list-filter-all = כל השרתים
-server-list-filter-favorites = מועדפים
-server-list-filter-recent = אחרונים
-server-list-sort-name = שם
-server-list-sort-host = מארח
-server-list-sort-last-connected = חיבור אחרון
-server-list-context-connect = התחברות
-server-list-context-edit = עריכה
-server-list-context-delete = מחיקה
-server-list-context-duplicate = שכפול
-server-list-context-copy-ip = העתקת IP
-server-list-context-add-to-group = הוספה לקבוצה
-server-list-context-remove-from-group = הסרה מהקבוצה
-server-group-uncategorized = ללא קטגוריה

## פרטי שרת
-server-detail-title = פרטי שרת
-server-detail-connect = התחברות
-server-detail-disconnect = ניתוק
-server-detail-edit = עריכה
-server-detail-delete = מחיקה
-server-detail-name = שם
-server-detail-host = מארח
-server-detail-port = פורט
-server-detail-username = שם משתמש
-server-detail-auth-method = אימות
-server-detail-auth-password = סיסמה
-server-detail-auth-key = מפתח SSH
-server-detail-auth-agent = סוכן SSH
-server-detail-group = קבוצה
-server-detail-tags = תגיות
-server-detail-color = צבע
-server-detail-notes = הערות
-server-detail-last-connected = חיבור אחרון
-server-detail-never-connected = לא היה מחובר
-server-detail-created = נוצר
-server-detail-updated = עודכן
-server-detail-connection-status = מצב חיבור
-server-detail-terminal = מסוף
-server-detail-sftp = SFTP
-server-detail-monitoring = מעקב
-server-detail-logs = יומנים

## דו-שיח שרת
-server-dialog-add-title = הוספת שרת
-server-dialog-edit-title = עריכת שרת
-server-dialog-name-label = שם השרת
-server-dialog-name-placeholder = לדוג' שרת ייצור
-server-dialog-host-label = מארח / כתובת IP
-server-dialog-host-placeholder = לדוג' 192.168.1.100 או server.example.com
-server-dialog-port-label = פורט
-server-dialog-port-default = ברירת מחדל: 22
-server-dialog-username-label = שם משתמש
-server-dialog-username-placeholder = לדוג' root או admin
-server-dialog-auth-label = שיטת אימות
-server-dialog-auth-password-label = סיסמה
-server-dialog-auth-password-placeholder = הזן סיסמה
-server-dialog-auth-key-label = מפתח פרטי
-server-dialog-auth-key-select = בחירת קובץ מפתח
-server-dialog-auth-key-passphrase = מילת מפתח (אופציונלי)
-server-dialog-auth-agent-label = שימוש בסוכן SSH
-server-dialog-group-label = קבוצה
-server-dialog-group-none = ללא קבוצה
-server-dialog-group-new = קבוצה חדשה...
-server-dialog-tags-label = תגיות
-server-dialog-tags-placeholder = הוסף תגיות, מופרדות בפסיקים
-server-dialog-color-label = צבע
-server-dialog-advanced = אפשרויות מתקדמות
-server-dialog-advanced-jump = מארח מגשר
-server-dialog-advanced-proxy = פרוקסי
-server-dialog-advanced-keepalive = מרווח keepalive
-server-dialog-save-and-connect = שמירה והתחברות
-server-dialog-save = שמירה
-server-dialog-test-connection = בדיקת חיבור
-server-dialog-test-success = החיבור הצליח!
-server-dialog-test-failed = החיבור נכשל: { $error }
-server-dialog-validation-required = { $field } נדרש
-server-dialog-validation-invalid-port = הפורט חייב להיות בין 1 ל-65535
-server-dialog-validation-invalid-host = כתובת מארח לא תקינה

## קבוצות
group-list-title = רשימת קבוצות
group-list-empty = לא נוצרו קבוצות
group-list-add = קבוצה חדשה
group-dialog-add-title = יצירת קבוצה
group-dialog-edit-title = עריכת קבוצה
group-dialog-name-label = שם הקבוצה
group-dialog-name-placeholder = לדוג' שרתי ייצור
group-dialog-parent-label = קבוצת אב (אופציונלי)
group-dialog-color-label = צבע הקבוצה
group-dialog-delete-confirm = למחוק את הקבוצה "{ $name }"?
group-dialog-delete-warning = השרתים בקבוצה זו יהפכו ללא קטגוריה.

## מסוף
-terminal-title = מסוף
-terminal-new-tab = לשונית חדשה
-terminal-close-tab = סגירת לשונית
-terminal-split-horizontal = פיצול אופקי
-terminal-split-vertical = פיצול אנכי
-terminal-copy = העתקה
-terminal-paste = הדבקה
-terminal-select-all = בחירת הכל
-terminal-find = חיפוש
-terminal-clear = ניקוי
-terminal-reset = איפוס
-terminal-zoom-in = הגדלה
-terminal-zoom-out = הקטנה
-terminal-zoom-reset = איפוס זום
-terminal-font-size = גודל גופן
-terminal-theme = ערכת נושא
-terminal-theme-dark = כהה
-terminal-theme-light = בהיר
-terminal-cursor-style = סגנון סמן
-terminal-cursor-blink = סמן מהבהב
-terminal-alt-screen = מסך חלופי
-terminal-mouse-reporting = דיווח עכבר
-terminal-disconnected = ההפעלה התנתקה. לחץ Enter להתחבר מחדש.
-terminal-reconnecting = מתחבר מחדש...
-terminal-command-palette = לוח פקודות
-terminal-sftp-open = פתיחת לוח SFTP

## SFTP
-sftp-title = העברת קבצים SFTP
-sftp-local = מקומי
-sftp-remote = מרוחק
-sftp-upload = העלאה
-sftp-download = הורדה
-sftp-delete = מחיקה
-sftp-rename = שינוי שם
-sftp-new-folder = תיקייה חדשה
-sftp-refresh = רענון
-sftp-transfer-progress = מעביר { $filename } ({ $progress }%)
-sftp-transfer-complete = ההעברה הושלמה: { $filename }
-sftp-transfer-error = ההעברה נכשלה: { $error }
-sftp-path-home = בית
-sftp-path-root = שורש
-sftp-context-upload = העלאה לכאן
-sftp-context-download = הורדה
-sftp-context-delete = מחיקה
-sftp-context-rename = שינוי שם
-sftp-context-properties = מאפיינים

## קטעי קוד
-snippets-title = קטעי קוד
-snippets-empty = לא נשמרו קטעי קוד
-snippets-add = קטע קוד חדש
-snippets-edit = עריכת קטע קוד
-snippets-delete = מחיקת קטע קוד
-snippets-run = הרצת קטע קוד
-snippets-name = שם
-snippets-content = תוכן
-snippets-variables = משתנים
-snippets-description = תיאור
-snippets-global = קטעי קוד גלובליים
-snippets-server-specific = ספציפי לשרת

## חיבור
-connection-connecting = מתחבר ל-{ $host }...
-connection-connected = מחובר ל-{ $host }
-connection-disconnected = מנותק מ-{ $host }
-connection-error = שגיאת חיבור: { $error }
-connection-auth-failed = האימות נכשל
-connection-host-key-changed = אזהרה: מפתח המארח המרוחק השתנה!
-connection-host-key-unknown = מפתח מארח לא ידוע. לאשר?
-connection-timeout = זמן החיבור תם
-connection-refused = החיבור נדחה
-connection-reset = החיבור אופס על ידי הצד השני
-connection-retrying = מנסה שוב... ({ $attempt }/{ $max })

## הגדרות
-settings-title = הגדרות
-settings-general = כללי
-settings-appearance = מראה
-settings-terminal = מסוף
-settings-ssh = SSH
-settings-advanced = מתקדם
-settings-language = שפה
-settings-language-auto = זיהוי אוטומטי
-settings-theme = ערכת נושא
-settings-theme-system = ברירת המחדל של המערכת
-settings-theme-dark = כהה
-settings-theme-light = בהיר
-settings-font-size = גודל גופן
-settings-font-family = גופן
-settings-show-tray-icon = הצגה במגש המערכת
-settings-minimize-to-tray = מזעור למגש
-settings-startup = הפעלה
-settings-startup-launch = הפעלה עם מערכת
-settings-startup-restore = שחזור ההפעלה האחרונה
-settings-updates = עדכונים
-settings-updates-check = בדיקת עדכונים
-settings-updates-auto = התקנת עדכונים אוטומטית
-settings-shortcuts = קיצורי מקלדת
-settings-shortcuts-new-tab = לשונית חדשה
-settings-shortcuts-close-tab = סגירת לשונית
-settings-shortcuts-next-tab = לשונית הבאה
-settings-shortcuts-prev-tab = לשונית קודמת
-settings-backup = גיבוי ושחזור
-settings-backup-export = ייצוא הגדרות
-settings-backup-import = ייבוא הגדרות
-settings-reset = איפוס להגדרות ברירת מחדל
-settings-reset-confirm = לאפס את כל ההגדרות לברירת המחדל?

## אבטחה
-security-unlock-title = שחרור EasySSH
-security-unlock-prompt = הזן את הסיסמה הראשית שלך להמשך
-security-unlock-button = שחרור
-security-forgot-password = שכחת סיסמה?
-security-set-password = הגדרת סיסמה ראשית
-security-change-password = שינוי סיסמה ראשית
-security-password-hint = רמז לסיסמה (אופציונלי)
-security-biometric-enable = הפעלת { $type }
-security-biometric-type-face = זיהוי פנים
-security-biometric-type-fingerprint = טביעת אצבע
-security-biometric-type-generic = אימות ביומטרי
-security-auto-lock = נעילה אוטומטית לאחר
-security-auto-lock-never = לעולם לא
-security-auto-lock-1m = דקה אחת
-security-auto-lock-5m = 5 דקות
-security-auto-lock-15m = 15 דקות
-security-auto-lock-1h = שעה אחת
-security-export-encrypted = הצפנת נתונים מיוצאים

## ייבוא/ייצוא
-import-title = ייבוא הגדרות
-import-source = מקור
-import-source-file = מקובץ
-import-source-url = מכתובת URL
-import-source-clipboard = מהלוח
-import-format = פורמט
-import-format-auto = זיהוי אוטומטי
-import-format-json = JSON
-import-format-csv = CSV
-import-format-ssh-config = תצורת SSH
-import-options = אפשרויות
-import-options-duplicate-skip = דילוג על כפילויות
-import-options-duplicate-replace = החלפת כפילויות
-import-options-duplicate-merge = מיזוג עם קיים
-import-preview = תצוגה מקדימה
-import-items-count = נמצאו { $count } פריטים
-import-success = הייבוא הצליח: יובאו { $count } פריטים
-import-error = הייבוא נכשל: { $error }
-export-title = ייצוא הגדרות
-export-format = פורמט ייצוא
-export-what = מה לייצא
-export-what-all = כל הנתונים
-export-what-servers = שרתים בלבד
-export-what-groups = קבוצות בלבד
-export-what-snippets = קטעי קוד בלבד
-export-encrypted = הצפנת ייצוא
-export-password = סיסמת ייצוא
-export-success = הייצוא הצליח
-export-error = הייצוא נכשל: { $error }

## מעקב
-monitoring-title = מעקב מערכת
-monitoring-cpu = מעבד
-monitoring-memory = זיכרון
-monitoring-disk = דיסק
-monitoring-network = רשת
-monitoring-processes = תהליכים
-monitoring-refresh = תדירות רענון
-monitoring-alert-threshold = סף התראות
-monitoring-history = היסטוריה
-monitoring-export-data = ייצוא נתונים

## תכונות צוות
-team-title = ניהול צוות
-team-members = חברים
-team-roles = תפקידים
-team-invite = הזמנת חבר
-team-remove = הסרת חבר
-team-role-owner = בעלים
-team-role-admin = מנהל
-team-role-member = חבר
-team-role-viewer = צופה
-team-sync = סנכרון צוות
-team-audit = יומן ביקורת
-team-shared-snippets = קטעי קוד משותפים

## התראות
-notify-connection-lost = החיבור אבד: { $server }
-notify-transfer-complete = העברת הקובץ הושלמה
-notify-transfer-failed = העברת הקובץ נכשלה
-notify-update-available = עדכון זמין: { $version }
-notify-copy-success = הועתק ללוח

## הודעות שגיאה
-error-generic = אירעה שגיאה
-error-not-found = המשאב לא נמצא
-error-permission = ההרשאה נדחתה
-error-network = שגיאת רשת
-error-timeout = זמן הבקשה תם
-error-invalid-input = קלט לא תקין
-error-database = שגיאת מסד נתונים
-error-crypto = שגיאת הצפנה
-error-ssh = שגיאת SSH: { $message }
-error-keychain = שגיאת מחסן מפתחות
-error-terminal = שגיאת מסוף
-error-import-failed = הייבוא נכשל
-error-export-failed = הייצוא נכשל
-error-file-not-found = הקובץ לא נמצא: { $path }
-error-invalid-key = מפתח SSH לא תקין
-error-connection-failed = החיבור נכשל

## נגישות
-a11y-terminal-label = לוח מסוף
-a11y-server-list-label = רשימת שרתים
-a11y-sidebar-label = ניווט צדדי
-a11y-close-dialog = סגירת דו-שיח
-a11y-expand-section = הרחבת מקטע
-a11y-collapse-section = כיווץ מקטע
-a11y-more-options = אפשרויות נוספות
-a11y-search-results = { $count } תוצאות חיפוש
-a11y-connection-status = מצב חיבור: { $status }
