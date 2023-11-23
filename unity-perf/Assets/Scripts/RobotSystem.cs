using Unity.Entities;
using Unity.Burst;
using Unity.Transforms;
using Unity.Mathematics;
using Unity.Collections;

public struct Robot : IComponentData
{
}

public struct RobotTarget : IComponentData { }
public struct AttachedToRobot : IComponentData { }
public struct CarryTarget : IComponentData { public Entity item; }
public struct DropTarget : IComponentData { public float3 position; }
public struct Cooldown : IComponentData { public float timeLeft; };

public partial struct RobotTargetSystem : ISystem
{
    EntityQuery unattached;
    EntityQuery emptyRobots;

    [BurstCompile]
    public void OnCreate(ref SystemState state)
    {
        unattached = new EntityQueryBuilder(state.WorldUpdateAllocator).WithNone<AttachedToRobot>().WithAll<RobotTarget>().Build(ref state);
        emptyRobots = new EntityQueryBuilder(state.WorldUpdateAllocator).WithNone<Cooldown, CarryTarget, Child, DropTarget>()
                      .WithAll<Robot>().Build(ref state);
    }


    [BurstCompile]
    public void OnUpdate(ref SystemState state)
    {
        var empties = emptyRobots.ToEntityArray(Allocator.TempJob);
        if (empties.Length == 0)
        {
            // no free robots this frame
            return;
        }

        var ecb = new EntityCommandBuffer(Allocator.TempJob);
        var parallel = ecb.AsParallelWriter();
        new AttachRobotJob { empties = empties, ecb = parallel }.ScheduleParallel(unattached);

        state.Dependency.Complete();

        ecb.Playback(state.EntityManager);

        ecb.Dispose();
    }

    [BurstCompile]
    partial struct AttachRobotJob : IJobEntity
    {
        public NativeArray<Entity> empties;
        public EntityCommandBuffer.ParallelWriter ecb;

        public void Execute([EntityIndexInQuery] int index, Entity entity)
        {
            if (index >= empties.Length)
            {
                return;
            }

            var robot = empties[index];
            ecb.AddComponent(index, robot, new CarryTarget { item = entity });
            ecb.AddComponent(index, entity, new AttachedToRobot { });
        }
    }
}

public partial struct RobotMoveToCarrySystem : ISystem
{
    private ComponentLookup<LocalToWorld> EntityPositions;
    private uint seedOffset;

    [BurstCompile]
    public void OnCreate(ref SystemState state)
    {
        state.RequireForUpdate<SpawnerComponent>();
        EntityPositions = state.GetComponentLookup<LocalToWorld>();
    }

    [BurstCompile]
    public void OnUpdate(ref SystemState state)
    {
        EntityPositions.Update(ref state);

        var spawner = SystemAPI.GetSingleton<SpawnerComponent>();
        var ecb = new EntityCommandBuffer(Allocator.TempJob);
        var job = new MoveToTargetJob();

        seedOffset += 200;

        job.EntityPositions = EntityPositions;
        job.ecb = ecb.AsParallelWriter();
        job.deltaTime = SystemAPI.Time.DeltaTime;
        job.moveSpeed = spawner.robotSpeed;
        job.maxSize = spawner.maxSize;
        job.seedOffset = seedOffset;

        job.ScheduleParallel();

        state.Dependency.Complete();

        ecb.Playback(state.EntityManager);
        ecb.Dispose();
    }

    [BurstCompile]
    partial struct MoveToTargetJob : IJobEntity
    {
        [ReadOnly]
        public ComponentLookup<LocalToWorld> EntityPositions;

        public uint seedOffset;

        public EntityCommandBuffer.ParallelWriter ecb;

        public float deltaTime;

        public float moveSpeed;

        public float maxSize;

        void Execute([EntityIndexInQuery] int index, Entity entity, ref LocalTransform transform, in CarryTarget target)
        {
            var item = target.item;
            if (EntityPositions.TryGetComponent(item, out LocalToWorld targetPosition))
            {
                var distanceSq = math.distancesq(targetPosition.Position, transform.Position);
                if (distanceSq < 0.1f)
                {
                    var rnd = Random.CreateFromIndex(seedOffset + (uint)index);
                    var pos = (rnd.NextFloat2() - new float2(0.5f, 0.5f)) * maxSize;
                    var pos2 = (rnd.NextFloat() - 0.5f) * 10;
                    ecb.AddComponent(index, entity, new DropTarget { position = new float3(pos.x, pos.y, 0f) });
                    ecb.RemoveComponent<CarryTarget>(index, entity);
                    ecb.AddComponent(index, item, new Parent { Value = entity });
                    ecb.SetComponent(index, item, LocalTransform.FromPositionRotationScale(new float3(0, 0.5f, 0), targetPosition.Rotation, 0.5f));
                    return;
                }



                var direction = math.normalize(targetPosition.Position - transform.Position);
                transform.Position = transform.Position + direction * moveSpeed * deltaTime;
            }
        }
    }
}

public partial struct RobotMoveToDropSystem : ISystem
{
    private BufferLookup<Child> childLookup;
    private uint seedOffset;

    [BurstCompile]
    public void OnCreate(ref SystemState state)
    {
        state.RequireForUpdate<SpawnerComponent>();
        childLookup = state.GetBufferLookup<Child>(true);
    }

    [BurstCompile]
    public void OnUpdate(ref SystemState state)
    {
        childLookup.Update(ref state);
        var spawner = SystemAPI.GetSingleton<SpawnerComponent>();
        var ecb = new EntityCommandBuffer(Allocator.TempJob);

        seedOffset += 200;
        var job = new MoveToPositionJob();
        job.moveSpeed = spawner.robotSpeed;
        job.deltaTime = SystemAPI.Time.DeltaTime;
        job.ChildLookup = childLookup;
        job.ecb = ecb.AsParallelWriter();
        job.seedOffset = seedOffset;

        job.ScheduleParallel();

        state.Dependency.Complete();

        ecb.Playback(state.EntityManager);
        ecb.Dispose();
    }

    [BurstCompile]
    partial struct MoveToPositionJob : IJobEntity
    {
        public float moveSpeed;
        public float deltaTime;
        public uint seedOffset;
        public EntityCommandBuffer.ParallelWriter ecb;
        [ReadOnly] public BufferLookup<Child> ChildLookup;

        void Execute([EntityIndexInQuery] int index, Entity entity, ref LocalTransform transform, in DropTarget target)
        {
            var targetPosition = target.position;
            var distanceSq = math.distancesq(targetPosition, transform.Position);
            if (distanceSq < 0.1f)
            {
                var rnd = Random.CreateFromIndex(seedOffset + (uint)index);

                if (ChildLookup.TryGetBuffer(entity, out var childBuffer))
                {
                    var children = childBuffer.AsNativeArray().Reinterpret<Entity>();
                    ecb.RemoveComponent<Parent>(index, children);
                    ecb.RemoveComponent<AttachedToRobot>(index, children);
                    var pos = (rnd.NextFloat2() - new float2(0.5f, 0.5f)) * 2f;
                    var finalPos = new float2(transform.Position.x, transform.Position.y) + pos;
                    var dropPos = new float3(finalPos.x, finalPos.y, 0f);
                    ecb.AddComponent(index, children, LocalTransform.FromPositionRotationScale(dropPos, transform.Rotation, 0.5f));
                }

                var waitTime = rnd.NextFloat() * 3;
                ecb.AddComponent(index, entity, new Cooldown { timeLeft = waitTime });
                ecb.RemoveComponent<DropTarget>(index, entity);
                return;
            }

            var direction = math.normalize(targetPosition - transform.Position);
            transform.Position = transform.Position + direction * moveSpeed * deltaTime;
        }
    }
}

public partial struct RobotCooldownSystem : ISystem
{
    [BurstCompile]
    public void OnUpdate(ref SystemState state)
    {
        var ecb = new EntityCommandBuffer(Allocator.TempJob);
        var job = new CooldownJob();
        job.ecb = ecb.AsParallelWriter();
        job.deltaTime = SystemAPI.Time.DeltaTime;

        job.ScheduleParallel();

        state.Dependency.Complete();

        ecb.Playback(state.EntityManager);
        ecb.Dispose();
    }

    [BurstCompile]
    partial struct CooldownJob : IJobEntity
    {
        public float deltaTime;
        public EntityCommandBuffer.ParallelWriter ecb;

        void Execute([EntityIndexInQuery] int index, Entity entity, ref Cooldown cooldown)
        {
            cooldown.timeLeft -= deltaTime;
            if (cooldown.timeLeft < 0)
            {
                ecb.RemoveComponent<Cooldown>(index, entity);
            }
        }
    }
}
