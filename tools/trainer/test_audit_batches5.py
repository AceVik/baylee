import unittest
import numpy as np
import torch
from audit_batches5 import sample_indices,padded_bytes,describe,measure


class AuditTests(unittest.TestCase):
    def test_sampling_covers_whole_epoch_deterministically(self):
        ids=sample_indices(5000,48)
        self.assertEqual(len(ids),48)
        self.assertEqual((ids[0],ids[-1]),(0,4999))
        self.assertEqual(ids,sample_indices(5000,48))
        self.assertEqual(sample_indices(2,48),[0,1])

    def test_padded_estimate_accounts_for_cross_lane_maxima(self):
        items=[dict(x=np.zeros((2,8),np.float32),z=np.array(True)),
               dict(x=np.zeros((4,3),np.float32),z=np.array(False))]
        self.assertEqual(padded_bytes(list(map(describe,items))),2*4*8*4+2)

    def test_padding_and_inactive_time_denominator(self):
        valid=torch.tensor([[True,True,False],[True,False,False]])
        masks=torch.tensor([[[True,True],[True,False],[True,True]],[[True,False],[False,False],[False,False]]])
        batch=dict(head=torch.zeros_like(valid,dtype=torch.long),step_valid=valid,is_decision=torch.tensor([[True,False,False],[True,False,False]]))
        for name in ('entity','edge','cand','pair'):batch[name+'_mask']=masks
        result=measure(batch)
        self.assertEqual(result['thinking_row_fraction_B_active_time'],.5)
        self.assertEqual(result['entities']['actual_entries'],4)
        self.assertEqual(result['entity_attention_E2']['actual_entries'],6)
        self.assertEqual(result['entity_attention_E2']['allocated_entries'],24)
        self.assertEqual(result['logical_batch_bytes'],sum(x.numel()*x.element_size() for x in batch.values()))
        self.assertLess(result['raw_batch_bytes'],result['logical_batch_bytes']) # shared mask storage counted once
        self.assertEqual(result['decision_head_counts']['0'],2)


if __name__=='__main__':unittest.main()
