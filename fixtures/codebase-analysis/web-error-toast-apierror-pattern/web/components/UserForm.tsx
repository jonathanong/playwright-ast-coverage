import { ApiError } from '@/lib/api-error';
import { toast } from '@/components/ui/toast';

export function UserForm() {
  const handleSubmit = async (data: FormData) => {
    try {
      await updateUser(data);
    } catch (err) {
      toast(err instanceof ApiError ? err.message : 'Something went wrong');
    }
  };

  return <form onSubmit={handleSubmit}><button type="submit">Save</button></form>;
}
