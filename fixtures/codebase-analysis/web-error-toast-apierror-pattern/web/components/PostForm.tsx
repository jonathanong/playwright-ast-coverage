import { toast } from '@/components/ui/toast';

export function PostForm() {
  const handleSubmit = async (data: FormData) => {
    try {
      await createPost(data);
    } catch (error) {
      toast(error.message);
    }
  };

  return <form onSubmit={handleSubmit}><button type="submit">Submit</button></form>;
}
