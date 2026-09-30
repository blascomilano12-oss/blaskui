// BlaskUI: profili ricordati su questa macchina per la pagina /accounts.
// Niente backend: le credenziali restano nel DB, qui solo nomi/email/avatar.
export type BlaskAccount = {
	name: string;
	email: string;
	profile_image_url?: string;
};

const KEY = 'blaskui-accounts';
const ADMIN_KEY = 'blaskui-admin-token';
const MAX = 8;

export const getBlaskAccounts = (): BlaskAccount[] => {
	try {
		const raw = localStorage.getItem(KEY);
		const list = raw ? (JSON.parse(raw) as BlaskAccount[]) : [];
		return Array.isArray(list) ? list.filter((a) => a?.email) : [];
	} catch {
		return [];
	}
};

export const rememberBlaskAccount = (u: {
	name?: string;
	email?: string;
	profile_image_url?: string;
	role?: string;
	token?: string;
}) => {
	if (!u?.email) return;
	try {
		const list = getBlaskAccounts().filter(
			(a) => a.email.toLowerCase() !== u.email!.toLowerCase()
		);
		list.unshift({
			name: u.name ?? u.email,
			email: u.email,
			profile_image_url: u.profile_image_url
		});
		localStorage.setItem(KEY, JSON.stringify(list.slice(0, MAX)));
		if (u.role === 'admin' && u.token) {
			localStorage.setItem(ADMIN_KEY, u.token);
		}
	} catch {
		// storage pieno o non disponibile: la pagina account resta vuota, niente crash
	}
};

export const forgetBlaskAccount = (email: string) => {
	try {
		localStorage.setItem(
			KEY,
			JSON.stringify(getBlaskAccounts().filter((a) => a.email.toLowerCase() !== email.toLowerCase()))
		);
	} catch {
		// niente da fare
	}
};

export const getBlaskAdminToken = (): string => {
	try {
		return localStorage.getItem(ADMIN_KEY) ?? '';
	} catch {
		return '';
	}
};

// Gradiente deterministico dal nome: avatar senza rete.
export const accountHue = (name: string): number => {
	let h = 0;
	for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) % 360;
	return h;
};

export const accountInitials = (name: string): string => {
	const parts = name.trim().split(/\s+/);
	if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
	return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
};
